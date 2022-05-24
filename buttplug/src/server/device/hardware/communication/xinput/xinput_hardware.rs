// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::xinput_device_comm_manager::{
  create_address,
  XInputConnectionTracker,
  XInputControllerIndex,
};
use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::{Endpoint, RawReading},
    ButtplugResultFuture,
  },
  server::device::{
    configuration::{ProtocolCommunicationSpecifier, XInputSpecifier},
    hardware::{
    HardwareEvent,
    HardwareConnector,
    GenericHardwareSpecializer,
    HardwareSpecializer,
    Hardware,
    HardwareInternal,
    HardwareReadCmd,
    HardwareSubscribeCmd,
    HardwareUnsubscribeCmd,
    HardwareWriteCmd,
    },
  },
  server::device::hardware::communication::HardwareSpecificError,
};
use async_trait::async_trait;
use byteorder::{LittleEndian, ReadBytesExt};
use futures::future::{self, BoxFuture};
use rusty_xinput::{XInputHandle, XInputUsageError};
use std::{
  fmt::{self, Debug},
  io::Cursor,
};
use tokio::sync::broadcast;

pub struct XInputHardwareConnector {
  index: XInputControllerIndex,
}

impl XInputHardwareConnector {
  pub fn new(index: XInputControllerIndex) -> Self {
    debug!("Emitting a new xbox device impl creator!");
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

  async fn connect(
    &mut self,
  ) -> Result<Box<dyn HardwareSpecializer>, ButtplugDeviceError> {
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
  connection_tracker: XInputConnectionTracker,
}

impl XInputHardware {
  pub fn new(index: XInputControllerIndex) -> Self {
    let (device_event_sender, _) = broadcast::channel(256);
    let connection_tracker = XInputConnectionTracker::default();
    connection_tracker.add_with_sender(index, device_event_sender.clone());
    Self {
      handle: rusty_xinput::XInputHandle::load_default().expect("The DLL should load as long as we're on windows, and we don't get here if we're not on windows."),
      index,
      event_sender: device_event_sender,
      connection_tracker,
    }
  }
}

impl HardwareInternal for XInputHardware {
  fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.event_sender.subscribe()
  }

  fn connected(&self) -> bool {
    self.connection_tracker.connected(self.index)
  }

  fn disconnect(&self) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    Box::pin(future::ready(Ok(())))
  }

  fn read_value(
    &self,
    _msg: &HardwareReadCmd,
  ) -> BoxFuture<'static, Result<RawReading, ButtplugDeviceError>> {
    let handle = self.handle.clone();
    let index = self.index;
    Box::pin(async move {
      let battery = handle
        .get_gamepad_battery_information(index as u32)
        .map_err(|e| {
          ButtplugDeviceError::from(HardwareSpecificError::XInputError(format!("{:?}", e)))
        })?;
      Ok(RawReading::new(index as u32, Endpoint::Rx, vec![battery.battery_level.0]))
    })
  }

  fn write_value(&self, msg: &HardwareWriteCmd) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let handle = self.handle.clone();
    let index = self.index;
    let data = msg.data.clone();
    Box::pin(async move {
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
            .into()
        })
    })
  }

  fn subscribe(&self, _msg: &HardwareSubscribeCmd) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    Box::pin(future::ready(Err(ButtplugDeviceError::UnhandledCommand("XInput hardware does not support subscribe".to_owned()))))
  }

  fn unsubscribe(&self, _msg: &HardwareUnsubscribeCmd) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    Box::pin(future::ready(Err(ButtplugDeviceError::UnhandledCommand("XInput hardware does not support unsubscribe".to_owned()))))
  }
}
