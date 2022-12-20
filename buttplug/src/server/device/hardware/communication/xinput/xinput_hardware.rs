// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::xinput_device_comm_manager::XInputControllerIndex;
use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::hardware::communication::HardwareSpecificError,
  server::device::{
    configuration::{ProtocolCommunicationSpecifier, XInputSpecifier},
    hardware::{
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
    },
  },
  util::async_manager,
};
use async_trait::async_trait;
use byteorder::{LittleEndian, ReadBytesExt};
use futures::future::{self, BoxFuture, FutureExt};
use rusty_xinput::{XInputHandle, XInputUsageError};
use std::{
  fmt::{self, Debug},
  io::Cursor,
  time::Duration,
};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

pub(super) fn create_address(index: XInputControllerIndex) -> String {
  index.to_string()
}

async fn check_gamepad_connectivity(
  index: XInputControllerIndex,
  sender: broadcast::Sender<HardwareEvent>,
  cancellation_token: CancellationToken,
) {
  let handle = rusty_xinput::XInputHandle::load_default()
    .expect("Always loads in windows, this shouldn't run elsewhere.");
  loop {
    // If we can't get state, assume we have disconnected.
    if handle.get_state(index as u32).is_err() {
      info!("XInput gamepad {} has disconnected.", index);
      // If this fails, we don't care because we're exiting anyways.
      let _ = sender.send(HardwareEvent::Disconnected(create_address(index)));
      return;
    }
    tokio::select! {
      _ = cancellation_token.cancelled() => return,
      _ = tokio::time::sleep(Duration::from_millis(500)) => continue
    }
  }
}

pub struct XInputHardwareConnector {
  index: XInputControllerIndex,
}

impl XInputHardwareConnector {
  pub fn new(index: XInputControllerIndex) -> Self {
    Self { index }
  }
}

impl Debug for XInputHardwareConnector {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("XInputHardwareConnector")
      .field("index", &self.index)
      .finish()
  }
}

#[async_trait]
impl HardwareConnector for XInputHardwareConnector {
  fn specifier(&self) -> ProtocolCommunicationSpecifier {
    ProtocolCommunicationSpecifier::XInput(XInputSpecifier::default())
  }

  async fn connect(&mut self) -> Result<Box<dyn HardwareSpecializer>, ButtplugDeviceError> {
    debug!("Emitting a new xbox device impl.");
    let hardware_internal = XInputHardware::new(self.index);
    let hardware = Hardware::new(
      &self.index.to_string(),
      &create_address(self.index),
      &[Endpoint::Tx, Endpoint::Rx],
      Box::new(hardware_internal),
    );
    Ok(Box::new(GenericHardwareSpecializer::new(hardware)))
  }
}

#[derive(Clone, Debug)]
pub struct XInputHardware {
  handle: XInputHandle,
  index: XInputControllerIndex,
  event_sender: broadcast::Sender<HardwareEvent>,
  cancellation_token: CancellationToken,
}

impl XInputHardware {
  pub fn new(index: XInputControllerIndex) -> Self {
    let (device_event_sender, _) = broadcast::channel(256);
    let token = CancellationToken::new();
    let child = token.child_token();
    let sender = device_event_sender.clone();
    async_manager::spawn(async move {
      check_gamepad_connectivity(index, sender, child).await;
    });
    Self {
      handle: rusty_xinput::XInputHandle::load_default().expect("The DLL should load as long as we're on windows, and we don't get here if we're not on windows."),
      index,
      event_sender: device_event_sender,
      cancellation_token: token,
    }
  }
}

impl HardwareInternal for XInputHardware {
  fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.event_sender.subscribe()
  }

  fn disconnect(&self) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Ok(())).boxed()
  }

  fn read_value(
    &self,
    _msg: &HardwareReadCmd,
  ) -> BoxFuture<'static, Result<HardwareReading, ButtplugDeviceError>> {
    let handle = self.handle.clone();
    let index = self.index;
    async move {
      let battery = handle
        .get_gamepad_battery_information(index as u32)
        .map_err(|e| {
          ButtplugDeviceError::from(HardwareSpecificError::XInputError(format!("{:?}", e)))
        })?;
      Ok(HardwareReading::new(
        Endpoint::Rx,
        &[battery.battery_level.0],
      ))
    }
    .boxed()
  }

  fn write_value(
    &self,
    msg: &HardwareWriteCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let handle = self.handle.clone();
    let index = self.index;
    let data = msg.data.clone();
    async move {
      let mut cursor = Cursor::new(data);
      let left_motor_speed = cursor
        .read_u16::<LittleEndian>()
        .expect("Packed in protocol, infallible");
      let right_motor_speed = cursor
        .read_u16::<LittleEndian>()
        .expect("Packed in protocol, infallible");
      handle
        .set_state(index as u32, left_motor_speed, right_motor_speed)
        .map_err(|e: XInputUsageError| {
          ButtplugDeviceError::from(HardwareSpecificError::XInputError(format!("{:?}", e)))
        })
    }
    .boxed()
  }

  fn subscribe(
    &self,
    _msg: &HardwareSubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "XInput hardware does not support subscribe".to_owned(),
    )))
    .boxed()
  }

  fn unsubscribe(
    &self,
    _msg: &HardwareUnsubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "XInput hardware does not support unsubscribe".to_owned(),
    )))
    .boxed()
  }
}

impl Drop for XInputHardware {
  fn drop(&mut self) {
    self.cancellation_token.cancel();
  }
}
