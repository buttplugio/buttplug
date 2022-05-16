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
    configuration::{ProtocolCommunicationSpecifier, ProtocolDeviceConfiguration, XInputSpecifier},
    hardware::{
    HardwareEvent,
    HardwareCreator,
    Hardware,
    HardwareInternal,
    HardwareReadCmd,
    HardwareSubscribeCmd,
    HardwareUnsubscribeCmd,
    HardwareWriteCmd,
    },
  },
  server::device::hardware::communication::ButtplugDeviceSpecificError,
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

pub struct XInputDeviceImplCreator {
  index: XInputControllerIndex,
}

impl XInputDeviceImplCreator {
  pub fn new(index: XInputControllerIndex) -> Self {
    debug!("Emitting a new xbox device impl creator!");
    Self { index }
  }
}

impl Debug for XInputDeviceImplCreator {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("XInputDeviceImplCreator")
      .field("index", &self.index)
      .finish()
  }
}

#[async_trait]
impl HardwareCreator for XInputDeviceImplCreator {
  fn specifier(&self) -> ProtocolCommunicationSpecifier {
    ProtocolCommunicationSpecifier::XInput(XInputSpecifier::default())
  }

  async fn try_create_hardware(
    &mut self,
    _protocol: ProtocolDeviceConfiguration,
  ) -> Result<Hardware, ButtplugError> {
    debug!("Emitting a new xbox device impl.");
    let device_impl_internal = XInputDeviceImpl::new(self.index);
    let device_impl = Hardware::new(
      &self.index.to_string(),
      &create_address(self.index),
      &[Endpoint::Tx],
      Box::new(device_impl_internal),
    );
    Ok(device_impl)
  }
}

#[derive(Clone, Debug)]
pub struct XInputDeviceImpl {
  handle: XInputHandle,
  index: XInputControllerIndex,
  event_sender: broadcast::Sender<HardwareEvent>,
  connection_tracker: XInputConnectionTracker,
}

impl XInputDeviceImpl {
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

impl HardwareInternal for XInputDeviceImpl {
  fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.event_sender.subscribe()
  }

  fn connected(&self) -> bool {
    self.connection_tracker.connected(self.index)
  }

  fn disconnect(&self) -> ButtplugResultFuture {
    Box::pin(future::ready(Ok(())))
  }

  fn read_value(
    &self,
    _msg: HardwareReadCmd,
  ) -> BoxFuture<'static, Result<RawReading, ButtplugError>> {
    panic!("We should never get here!");
  }

  fn write_value(&self, msg: HardwareWriteCmd) -> ButtplugResultFuture {
    let handle = self.handle.clone();
    let index = self.index;
    Box::pin(async move {
      let mut cursor = Cursor::new(msg.data);
      let left_motor_speed = cursor
        .read_u16::<LittleEndian>()
        .expect("Packed in protocol, infallible");
      let right_motor_speed = cursor
        .read_u16::<LittleEndian>()
        .expect("Packed in protocol, infallible");
      handle
        .set_state(index as u32, left_motor_speed, right_motor_speed)
        .map_err(|e: XInputUsageError| {
          ButtplugDeviceError::from(ButtplugDeviceSpecificError::XInputError(format!("{:?}", e)))
            .into()
        })
    })
  }

  fn subscribe(&self, _msg: HardwareSubscribeCmd) -> ButtplugResultFuture {
    panic!("We should never get here!");
  }

  fn unsubscribe(&self, _msg: HardwareUnsubscribeCmd) -> ButtplugResultFuture {
    panic!("We should never get here!");
  }
}
