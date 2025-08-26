// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
  },
  server::device::{
    configuration::{ProtocolCommunicationSpecifier, UserDeviceDefinition, UserDeviceIdentifier},
    hardware::Hardware,
    protocol::{
      generic_protocol_initializer_setup,
      ProtocolHandler,
      ProtocolIdentifier,
      ProtocolInitializer,
      kiiroo_v21::KiirooV21,
    },
  },
};
use async_trait::async_trait;
use std::sync::{
  Arc,
};

generic_protocol_initializer_setup!(KiirooKeonWifi, "kiiroo-keonwifi");

#[derive(Default)]
pub struct KiirooKeonWifiInitializer {}

#[async_trait]
impl ProtocolInitializer for KiirooKeonWifiInitializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    _: &UserDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(KiirooV21::default()))
  }
}
