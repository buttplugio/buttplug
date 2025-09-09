// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::Hardware,
  protocol::{
    generic_protocol_initializer_setup,
    ProtocolHandler,
    ProtocolIdentifier,
    ProtocolInitializer,
  },
  protocol_impl::kiiroo_v21::KiirooV21,
};
use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::{
  DeviceDefinition,
  ProtocolCommunicationSpecifier,
  UserDeviceIdentifier,
};
use std::sync::{
  Arc,
};

generic_protocol_initializer_setup!(KiirooV3, "kiiroo-v3");

#[derive(Default)]
pub struct KiirooV3Initializer {}

#[async_trait]
impl ProtocolInitializer for KiirooV3Initializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    _: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(KiirooV21::default()))
  }
}
