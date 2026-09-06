// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Hardware connector and hardware implementation for SDL3 gamepads.

use super::sdl_task::{RUMBLE_DURATION_MS, SdlGamepadBackend, SdlOpenedGamepad, SdlTaskError};
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
  Endpoint,
  ProtocolCommunicationSpecifier,
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
}

impl SdlGamepadHardwareConnector {
  pub(crate) fn new(
    backend: Arc<dyn SdlGamepadBackend>,
    id: JoystickId,
    name: String,
    address: String,
  ) -> Self {
    Self {
      backend,
      id,
      name,
      address,
    }
  }
}

impl Debug for SdlGamepadHardwareConnector {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("SdlGamepadHardwareConnector")
      .field("id", &self.id.0)
      .field("name", &self.name)
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
    let opened = self
      .backend
      .open(self.id)
      .await
      .map_err(|e| hardware_error("open", e))?;
    let hardware_internal = SdlGamepadHardware::new(opened, self.address.clone());
    let hardware = Hardware::new(
      &self.name,
      &self.address,
      &[Endpoint::Tx],
      &None,
      false,
      Box::new(hardware_internal),
    );
    Ok(Box::new(GenericHardwareSpecializer::new(hardware)))
  }
}

/// Watches the backend's removal signal and emits Disconnected on the
/// device's broadcast event stream (pattern from the XInput manager).
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
  event_sender: broadcast::Sender<HardwareEvent>,
  cancellation_token: CancellationToken,
}

impl SdlGamepadHardware {
  fn new(opened: Arc<dyn SdlOpenedGamepad>, address: String) -> Self {
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
    _msg: &HardwareReadCmd,
  ) -> BoxFuture<'static, Result<HardwareReading, ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "SDL gamepad hardware does not support read".to_owned(),
    )))
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
    async move {
      // The protocol guarantees 4 bytes (two u16 LE motor speeds), but a
      // short read must error, not panic.
      let mut cursor = Cursor::new(data);
      let (low, high) = match (
        cursor.read_u16::<LittleEndian>(),
        cursor.read_u16::<LittleEndian>(),
      ) {
        (Ok(low), Ok(high)) => (low, high),
        _ => {
          return Err(ButtplugDeviceError::DeviceCommunicationError(
            "SDL gamepad write payload must be 4 bytes (two u16 LE motor speeds)".to_owned(),
          ));
        }
      };
      opened
        .rumble(low, high, RUMBLE_DURATION_MS)
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
    closed: Mutex<usize>,
    removed_tx: watch::Sender<bool>,
  }

  #[async_trait]
  impl SdlOpenedGamepad for MockOpenedGamepad {
    async fn rumble(&self, low: u16, high: u16, duration_ms: u32) -> Result<(), SdlTaskError> {
      self
        .rumble_calls
        .lock()
        .unwrap()
        .push((low, high, duration_ms));
      Ok(())
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

    async fn open(&self, _id: JoystickId) -> Result<Arc<dyn SdlOpenedGamepad>, SdlTaskError> {
      self
        .opened
        .lock()
        .unwrap()
        .clone()
        .map(|pad| pad as Arc<dyn SdlOpenedGamepad>)
        .ok_or_else(|| SdlTaskError::Open("no mock gamepad".to_owned()))
    }
  }

  async fn connect_mock_hardware() -> (Arc<MockOpenedGamepad>, Hardware, Arc<MockBackend>) {
    let mock_pad = Arc::new(MockOpenedGamepad {
      rumble_calls: Mutex::new(Vec::new()),
      closed: Mutex::new(0),
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
    assert_eq!(hardware.endpoints(), &[Endpoint::Tx]);
    (mock_pad, hardware, backend)
  }

  #[tokio::test]
  async fn hardware_write_value_forwards_motor_pair() {
    let (mock_pad, hardware, _backend) = connect_mock_hardware().await;

    // 1:1 passthrough of the two parsed u16 LE values.
    hardware
      .write_value(&HardwareWriteCmd::new(
        &[uuid::Uuid::new_v4()],
        Endpoint::Tx,
        vec![0x00, 0x80, 0xff, 0x7f],
        false,
      ))
      .await
      .expect("write should succeed");
    assert_eq!(
      *mock_pad.rumble_calls.lock().unwrap(),
      vec![(0x8000, 0x7fff, RUMBLE_DURATION_MS)]
    );

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
