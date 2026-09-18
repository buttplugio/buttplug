// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Hardware connector and hardware implementation for SDL3 gamepads.

use super::sdl_task::{
  RUMBLE_DURATION_MS,
  SdlGamepadBackend,
  SdlOpenedGamepad,
  SdlRumbleCapabilities,
  SdlRumbleState,
  SdlTaskError,
};
use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server::device::hardware::{
  GenericHardwareSpecializer,
  Hardware,
  HardwareConnector,
  HardwareEvent,
  HardwareInternal,
  HardwareReadCmd,
  HardwareReading,
  HardwareSpecializer,
  HardwareSubscribeCmd,
  HardwareUnsubscribeCmd,
  HardwareWriteCmd,
  communication::HardwareSpecificError,
};
use buttplug_server_device_config::{
  DeviceDefinitionSelection,
  Endpoint,
  ProtocolCommunicationSpecifier,
  SDL_PROTOCOL_NAME,
  SDL_RUMBLE_AND_TRIGGERS_SELECTOR,
  SDL_TRIGGERS_ONLY_SELECTOR,
  SdlGamepadSpecifier,
};
use byteorder::{LittleEndian, ReadBytesExt};
use futures::future::{self, BoxFuture, FutureExt};
use sdl3::joystick::JoystickId;
use std::{
  fmt::{self, Debug},
  io::Cursor,
  sync::Arc,
};
use tokio::sync::{broadcast, watch};
use tokio_util::sync::CancellationToken;

pub(crate) struct SdlGamepadHardwareConnector {
  backend: Arc<dyn SdlGamepadBackend>,
  id: JoystickId,
  name: String,
  address: String,
  capabilities: SdlRumbleCapabilities,
}

impl SdlGamepadHardwareConnector {
  pub(crate) fn new(
    backend: Arc<dyn SdlGamepadBackend>,
    id: JoystickId,
    name: String,
    address: String,
    capabilities: SdlRumbleCapabilities,
  ) -> Self {
    Self {
      backend,
      id,
      name,
      address,
      capabilities,
    }
  }
}

impl Debug for SdlGamepadHardwareConnector {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("SdlGamepadHardwareConnector")
      .field("id", &self.id.raw())
      .field("name", &self.name)
      .field("capabilities", &self.capabilities)
      .finish()
  }
}

pub(crate) fn hardware_error(operation: &str, e: SdlTaskError) -> ButtplugDeviceError {
  ButtplugDeviceError::from(ButtplugDeviceError::DeviceSpecificError(
    HardwareSpecificError::HardwareSpecificError(
      "SdlGamepad".to_string(),
      format!("{operation}: {e}"),
    )
    .to_string(),
  ))
}

#[async_trait]
impl HardwareConnector for SdlGamepadHardwareConnector {
  fn specifier(&self) -> ProtocolCommunicationSpecifier {
    ProtocolCommunicationSpecifier::SdlGamepad(SdlGamepadSpecifier::default())
  }

  async fn connect(&mut self) -> Result<Box<dyn HardwareSpecializer>, ButtplugDeviceError> {
    debug!("Emitting a new SDL gamepad device impl ({})", self.address);
    let (opened, caps) = self
      .backend
      .open(self.id)
      .await
      .map_err(|e| hardware_error("open", e))?;
    let base_identifier = match (caps.rumble, caps.trigger_rumble) {
      (true, true) => Some(SDL_RUMBLE_AND_TRIGGERS_SELECTOR),
      (true, false) => None,
      (false, true) => Some(SDL_TRIGGERS_ONLY_SELECTOR),
      (false, false) => {
        opened.close_now();
        return Err(hardware_error(
          "open",
          SdlTaskError::NoRumbleCapability(self.id),
        ));
      }
    };
    let hardware_internal = SdlGamepadHardware::new(opened, self.address.clone(), caps);
    let hardware = Hardware::new(
      &self.name,
      &self.address,
      &[Endpoint::Tx, Endpoint::Rx],
      &None,
      false,
      Box::new(hardware_internal),
    )
    .with_definition_selection(DeviceDefinitionSelection::new(
      SDL_PROTOCOL_NAME,
      base_identifier,
      &self.name,
    ));
    Ok(Box::new(GenericHardwareSpecializer::new(hardware)))
  }
}

/// Watches the backend's removal signal and emits Disconnected on the
/// device's broadcast event stream.
async fn watch_removal(
  mut removed: watch::Receiver<bool>,
  sender: broadcast::Sender<HardwareEvent>,
  address: String,
  cancellation_token: CancellationToken,
) {
  loop {
    tokio::select! {
      _ = cancellation_token.cancelled() => return,
      changed = removed.changed() => {
        if changed.is_err() {
          // Sender dropped along with the SDL-thread state; treat as removed.
          break;
        }
        if *removed.borrow() {
          break;
        }
      }
    }
  }
  info!("SDL gamepad {} has disconnected.", address);
  // If this fails, nobody was listening; nothing else to do.
  let _ = sender.send(HardwareEvent::Disconnected(address));
}

pub(crate) struct SdlGamepadHardware {
  opened: Option<Arc<dyn SdlOpenedGamepad>>,
  capabilities: SdlRumbleCapabilities,
  event_sender: broadcast::Sender<HardwareEvent>,
  cancellation_token: CancellationToken,
}

impl SdlGamepadHardware {
  fn new(
    opened: Arc<dyn SdlOpenedGamepad>,
    address: String,
    capabilities: SdlRumbleCapabilities,
  ) -> Self {
    let (device_event_sender, _) = broadcast::channel(256);
    let token = CancellationToken::new();
    let child = token.child_token();
    let sender = device_event_sender.clone();
    let removed = opened.removed();
    let watch_address = address.clone();
    buttplug_core::spawn!("SdlGamepadHardware removal watch", async move {
      watch_removal(removed, sender, watch_address, child).await;
    });
    Self {
      opened: Some(opened),
      capabilities,
      event_sender: device_event_sender,
      cancellation_token: token,
    }
  }

  fn close_opened(&self) {
    if let Some(opened) = &self.opened {
      opened.close_now();
    }
  }
}

impl HardwareInternal for SdlGamepadHardware {
  fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.event_sender.subscribe()
  }

  fn disconnect(&self) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    // Graceful path: tell the SDL thread to close the gamepad and wait for
    // it. (Drop uses the fire-and-forget close since it cannot await.)
    if let Some(opened) = &self.opened {
      let opened = opened.clone();
      return async move { opened.close().await.map_err(|e| hardware_error("close", e)) }.boxed();
    }
    future::ready(Ok(())).boxed()
  }

  fn read_value(
    &self,
    msg: &HardwareReadCmd,
  ) -> BoxFuture<'static, Result<HardwareReading, ButtplugDeviceError>> {
    if msg.endpoint() != Endpoint::Rx {
      return future::ready(Err(ButtplugDeviceError::UnhandledCommand(
        "SDL gamepad hardware only supports battery reads on rx".to_owned(),
      )))
      .boxed();
    }
    let Some(opened) = &self.opened else {
      return future::ready(Err(ButtplugDeviceError::DeviceCommunicationError(
        "SDL gamepad hardware is already closed".to_owned(),
      )))
      .boxed();
    };
    let opened = opened.clone();
    async move {
      let percent = opened
        .battery_level()
        .await
        .map_err(|e| hardware_error("battery", e))?;
      Ok(HardwareReading::new(Endpoint::Rx, &[percent]))
    }
    .boxed()
  }

  fn write_value(
    &self,
    msg: &HardwareWriteCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let Some(opened) = &self.opened else {
      return future::ready(Err(ButtplugDeviceError::DeviceCommunicationError(
        "SDL gamepad hardware is already closed".to_owned(),
      )))
      .boxed();
    };
    let opened = opened.clone();
    let data = msg.data().clone();
    let caps = self.capabilities;
    async move {
      if data.len() != 8 {
        return Err(ButtplugDeviceError::DeviceCommunicationError(
          "SDL gamepad write payload must be 8 bytes (four u16 LE channel values)".to_owned(),
        ));
      }
      let mut cursor = Cursor::new(data);
      let state = match (
        cursor.read_u16::<LittleEndian>(),
        cursor.read_u16::<LittleEndian>(),
        cursor.read_u16::<LittleEndian>(),
        cursor.read_u16::<LittleEndian>(),
      ) {
        (Ok(low), Ok(high), Ok(left_trigger), Ok(right_trigger)) => SdlRumbleState {
          low,
          high,
          left_trigger,
          right_trigger,
        },
        _ => {
          return Err(ButtplugDeviceError::DeviceCommunicationError(
            "SDL gamepad write payload must be 8 bytes (four u16 LE channel values)".to_owned(),
          ));
        }
      };
      let [low, high, left, right] = state.slots();
      if !caps.rumble && (low != 0 || high != 0) {
        return Err(ButtplugDeviceError::DeviceCommunicationError(
          "SDL gamepad does not support main rumble".to_owned(),
        ));
      }
      if !caps.trigger_rumble && (left != 0 || right != 0) {
        return Err(ButtplugDeviceError::DeviceCommunicationError(
          "SDL gamepad does not support trigger rumble".to_owned(),
        ));
      }
      opened
        .set_rumble_state(state, RUMBLE_DURATION_MS)
        .await
        .map_err(|e| hardware_error("rumble", e))
    }
    .boxed()
  }

  fn subscribe(
    &self,
    _msg: &HardwareSubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "SDL gamepad hardware does not support subscribe".to_owned(),
    )))
    .boxed()
  }

  fn unsubscribe(
    &self,
    _msg: &HardwareUnsubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "SDL gamepad hardware does not support unsubscribe".to_owned(),
    )))
    .boxed()
  }
}

impl Drop for SdlGamepadHardware {
  fn drop(&mut self) {
    self.cancellation_token.cancel();
    self.close_opened();
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    sdl_comm_manager::create_address,
    sdl_task::{SdlGamepadDesc, SdlOpenedGamepad, SdlTaskError, joystick_id},
  };
  use std::sync::Mutex;

  /// Pure outer-seam mock: records rumble/close calls, signals removal.
  #[derive(Debug)]
  struct MockOpenedGamepad {
    rumble_calls: Mutex<Vec<(u16, u16, u32)>>,
    trigger_calls: Mutex<Vec<(u16, u16, u32)>>,
    rumble_attempts: Mutex<Vec<(u16, u16, u32)>>,
    trigger_attempts: Mutex<Vec<(u16, u16, u32)>>,
    fail: Mutex<bool>,
    commands: Mutex<usize>,
    caps: SdlRumbleCapabilities,
    closed: Mutex<usize>,
    battery: Mutex<Result<u8, SdlTaskError>>,
    removed_tx: watch::Sender<bool>,
  }

  #[async_trait]
  impl SdlOpenedGamepad for MockOpenedGamepad {
    async fn set_rumble_state(
      &self,
      state: SdlRumbleState,
      duration_ms: u32,
    ) -> Result<(), SdlTaskError> {
      *self.commands.lock().unwrap() += 1;
      let fail = *self.fail.lock().unwrap();
      if self.caps.rumble {
        self
          .rumble_attempts
          .lock()
          .unwrap()
          .push((state.low, state.high, duration_ms));
        if !fail {
          self
            .rumble_calls
            .lock()
            .unwrap()
            .push((state.low, state.high, duration_ms));
        }
      }
      if self.caps.trigger_rumble {
        self.trigger_attempts.lock().unwrap().push((
          state.left_trigger,
          state.right_trigger,
          duration_ms,
        ));
        if !fail {
          self.trigger_calls.lock().unwrap().push((
            state.left_trigger,
            state.right_trigger,
            duration_ms,
          ));
        }
      }
      if fail {
        Err(SdlTaskError::Rumble("mock failure".to_owned()))
      } else {
        Ok(())
      }
    }

    async fn battery_level(&self) -> Result<u8, SdlTaskError> {
      self.battery.lock().unwrap().clone()
    }

    async fn close(&self) -> Result<(), SdlTaskError> {
      *self.closed.lock().unwrap() += 1;
      let _ = self.removed_tx.send(true);
      Ok(())
    }

    fn close_now(&self) {
      *self.closed.lock().unwrap() += 1;
      let _ = self.removed_tx.send(true);
    }

    fn removed(&self) -> watch::Receiver<bool> {
      self.removed_tx.subscribe()
    }
  }

  struct MockBackend {
    opened: Mutex<Option<Arc<MockOpenedGamepad>>>,
    gamepads: Mutex<Vec<SdlGamepadDesc>>,
  }

  #[async_trait]
  impl SdlGamepadBackend for MockBackend {
    fn initialized(&self) -> bool {
      true
    }

    async fn gamepads(&self) -> Result<Vec<SdlGamepadDesc>, SdlTaskError> {
      Ok(self.gamepads.lock().unwrap().clone())
    }

    async fn open(
      &self,
      _id: JoystickId,
    ) -> Result<(Arc<dyn SdlOpenedGamepad>, SdlRumbleCapabilities), SdlTaskError> {
      self
        .opened
        .lock()
        .unwrap()
        .clone()
        .map(|pad| {
          let caps = pad.caps;
          (pad as Arc<dyn SdlOpenedGamepad>, caps)
        })
        .ok_or_else(|| SdlTaskError::Open("no mock gamepad".to_owned()))
    }
  }

  async fn connect_mock_hardware() -> (Arc<MockOpenedGamepad>, Hardware, Arc<MockBackend>) {
    connect_mock_caps(SdlRumbleCapabilities {
      rumble: true,
      trigger_rumble: false,
    })
    .await
  }

  async fn connect_mock_caps(
    caps: SdlRumbleCapabilities,
  ) -> (Arc<MockOpenedGamepad>, Hardware, Arc<MockBackend>) {
    let mock_pad = Arc::new(MockOpenedGamepad {
      caps,
      rumble_calls: Mutex::new(Vec::new()),
      trigger_calls: Mutex::new(Vec::new()),
      rumble_attempts: Mutex::new(Vec::new()),
      trigger_attempts: Mutex::new(Vec::new()),
      fail: Mutex::new(false),
      commands: Mutex::new(0),
      closed: Mutex::new(0),
      battery: Mutex::new(Ok(80)),
      removed_tx: watch::channel(false).0,
    });
    let backend = Arc::new(MockBackend {
      opened: Mutex::new(Some(mock_pad.clone())),
      gamepads: Mutex::new(Vec::new()),
    });
    let mut connector = SdlGamepadHardwareConnector::new(
      backend.clone(),
      joystick_id(21),
      "SDL Gamepad".to_owned(),
      create_address(joystick_id(21)),
      caps,
    );
    assert_eq!(
      connector.specifier(),
      ProtocolCommunicationSpecifier::SdlGamepad(SdlGamepadSpecifier::default())
    );
    let mut specializer = connector.connect().await.expect("connect should succeed");
    let hardware = specializer
      .specialize(&[connector.specifier()])
      .await
      .expect("specialize should succeed");
    assert_eq!(hardware.name(), "SDL Gamepad");
    assert_eq!(hardware.address(), "sdl-gamepad-21");
    assert_eq!(hardware.endpoints(), &[Endpoint::Tx, Endpoint::Rx]);
    (mock_pad, hardware, backend)
  }

  #[tokio::test]
  async fn hardware_write_value_forwards_motor_pair() {
    let (mock_pad, hardware, _backend) = connect_mock_hardware().await;

    // Main-only pads receive only their supported pair.
    hardware
      .write_value(&HardwareWriteCmd::new(
        &[uuid::Uuid::new_v4()],
        Endpoint::Tx,
        vec![0x00, 0x80, 0xff, 0x7f, 0, 0, 0, 0],
        false,
      ))
      .await
      .expect("write should succeed");
    assert_eq!(
      *mock_pad.rumble_calls.lock().unwrap(),
      vec![(0x8000, 0x7fff, RUMBLE_DURATION_MS)]
    );

    assert!(mock_pad.trigger_attempts.lock().unwrap().is_empty());
    assert_eq!(*mock_pad.commands.lock().unwrap(), 1);

    // Short payloads error rather than panic.
    let err = hardware
      .write_value(&HardwareWriteCmd::new(
        &[uuid::Uuid::new_v4()],
        Endpoint::Tx,
        vec![0x00, 0x80],
        false,
      ))
      .await;
    assert!(err.is_err());
    assert_eq!(mock_pad.rumble_calls.lock().unwrap().len(), 1);

    // Other unsupported commands error as unhandled.
    assert!(
      hardware
        .read_value(&HardwareReadCmd::new(
          uuid::Uuid::new_v4(),
          Endpoint::Tx,
          0,
          0
        ))
        .await
        .is_err()
    );
  }

  #[tokio::test]
  async fn sdl_hardware_battery_rx_read() {
    let (mock_pad, hardware, _backend) = connect_mock_hardware().await;
    *mock_pad.battery.lock().unwrap() = Ok(64);
    let reading = hardware
      .read_value(&HardwareReadCmd::new(
        uuid::Uuid::new_v4(),
        Endpoint::Rx,
        1,
        0,
      ))
      .await
      .expect("battery read should succeed");
    assert_eq!(*reading.endpoint(), Endpoint::Rx);
    assert_eq!(reading.data(), &[64]);

    *mock_pad.battery.lock().unwrap() = Err(SdlTaskError::Battery("nope".to_owned()));
    let error = hardware
      .read_value(&HardwareReadCmd::new(
        uuid::Uuid::new_v4(),
        Endpoint::Rx,
        1,
        0,
      ))
      .await
      .expect_err("battery failure should be returned");
    assert!(error.to_string().contains("battery"));
  }

  #[tokio::test]
  async fn sdl_hardware_read_rejects_non_rx_endpoint() {
    let (_mock_pad, hardware, _backend) = connect_mock_hardware().await;
    for endpoint in [Endpoint::Tx, Endpoint::RxBLEBattery] {
      assert!(
        hardware
          .read_value(&HardwareReadCmd::new(uuid::Uuid::new_v4(), endpoint, 1, 0))
          .await
          .is_err()
      );
    }
  }

  #[tokio::test]
  async fn sdl_hardware_endpoints_include_rx() {
    let (_mock_pad, hardware, _backend) = connect_mock_hardware().await;
    assert_eq!(hardware.endpoints(), &[Endpoint::Tx, Endpoint::Rx]);
  }

  fn packet(data: Vec<u8>) -> HardwareWriteCmd {
    HardwareWriteCmd::new(&[uuid::Uuid::new_v4()], Endpoint::Tx, data, false)
  }

  #[tokio::test]
  async fn sdl_hardware_packet_validation() {
    for caps in [
      SdlRumbleCapabilities {
        rumble: true,
        trigger_rumble: false,
      },
      SdlRumbleCapabilities {
        rumble: false,
        trigger_rumble: true,
      },
    ] {
      let (pad, hardware, _) = connect_mock_caps(caps).await;
      for bytes in [vec![0; 4], vec![0; 9]] {
        assert!(hardware.write_value(&packet(bytes)).await.is_err());
      }
      let mut unsupported = vec![0; 8];
      unsupported[if caps.rumble { 4 } else { 0 }] = 1;
      let error = hardware
        .write_value(&packet(unsupported))
        .await
        .unwrap_err()
        .to_string();
      assert!(error.contains(if caps.rumble {
        "does not support trigger rumble"
      } else {
        "does not support main rumble"
      }));
      assert!(pad.rumble_calls.lock().unwrap().is_empty());
      assert!(pad.trigger_calls.lock().unwrap().is_empty());
      assert_eq!(*pad.commands.lock().unwrap(), 0);
    }
  }

  #[tokio::test]
  async fn sdl_backend_supported_pair_dispatch() {
    for caps in [
      SdlRumbleCapabilities {
        rumble: true,
        trigger_rumble: false,
      },
      SdlRumbleCapabilities {
        rumble: false,
        trigger_rumble: true,
      },
      SdlRumbleCapabilities {
        rumble: true,
        trigger_rumble: true,
      },
    ] {
      let (pad, hardware, _) = connect_mock_caps(caps).await;
      let state = SdlRumbleState {
        low: if caps.rumble { 500 } else { 0 },
        right_trigger: if caps.trigger_rumble { 700 } else { 0 },
        ..Default::default()
      };
      hardware
        .write_value(&packet(
          state
            .slots()
            .into_iter()
            .flat_map(u16::to_le_bytes)
            .collect(),
        ))
        .await
        .unwrap();
      hardware.write_value(&packet(vec![0; 8])).await.unwrap();
      assert_eq!(
        pad.rumble_calls.lock().unwrap().len(),
        if caps.rumble { 2 } else { 0 }
      );
      assert_eq!(
        pad.trigger_calls.lock().unwrap().len(),
        if caps.trigger_rumble { 2 } else { 0 }
      );
      if caps.rumble {
        assert_eq!(
          *pad.rumble_calls.lock().unwrap(),
          vec![(500, 0, RUMBLE_DURATION_MS), (0, 0, RUMBLE_DURATION_MS)]
        );
      }
      if caps.trigger_rumble {
        assert_eq!(
          *pad.trigger_calls.lock().unwrap(),
          vec![(0, 700, RUMBLE_DURATION_MS), (0, 0, RUMBLE_DURATION_MS)]
        );
      }
      assert_eq!(*pad.commands.lock().unwrap(), 2);
      *pad.fail.lock().unwrap() = true;
      assert!(hardware.write_value(&packet(vec![0; 8])).await.is_err());
      assert_eq!(
        pad.rumble_attempts.lock().unwrap().len(),
        if caps.rumble { 3 } else { 0 }
      );
      assert_eq!(
        pad.trigger_attempts.lock().unwrap().len(),
        if caps.trigger_rumble { 3 } else { 0 }
      );
    }
  }

  #[tokio::test]
  async fn hardware_close_and_drop_close_backend_handle() {
    // Explicit disconnect closes the backend handle.
    {
      let (mock_pad, hardware, _backend) = connect_mock_hardware().await;
      hardware
        .disconnect()
        .await
        .expect("disconnect should succeed");
      assert_eq!(*mock_pad.closed.lock().unwrap(), 1);
    }

    // Dropping the hardware also closes the backend handle.
    {
      let (mock_pad, hardware, _backend) = connect_mock_hardware().await;
      drop(hardware);
      assert!(
        *mock_pad.closed.lock().unwrap() >= 1,
        "drop must close the backend handle"
      );
    }
  }

  #[tokio::test]
  async fn hardware_removal_emits_disconnected_event() {
    let (mock_pad, hardware, _backend) = connect_mock_hardware().await;
    let mut event_stream = hardware.event_stream();

    // Simulate SDL-side removal.
    let _ = mock_pad.removed_tx.send(true);

    let event = tokio::time::timeout(std::time::Duration::from_secs(5), event_stream.recv())
      .await
      .expect("disconnected event must arrive within timeout")
      .expect("event stream must stay live");
    match event {
      HardwareEvent::Disconnected(address) => assert_eq!(address, "sdl-gamepad-21"),
      other => panic!("expected Disconnected, got {other:?}"),
    }
  }
}
