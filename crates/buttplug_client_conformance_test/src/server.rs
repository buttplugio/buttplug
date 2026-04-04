// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device_definitions::conformance_device_definitions;
use crate::device_manager::{
  ConformanceDeviceCommunicationManagerBuilder, ConformanceDeviceHandle,
};
use buttplug_server::device::ServerDeviceManagerBuilder;
use buttplug_server::{ButtplugServer, ButtplugServerBuilder, ButtplugServerError};
use buttplug_server_device_config::load_protocol_configs;

/// Builds a ButtplugServer with the conformance device manager and test devices
pub fn build_conformance_server(
  max_ping_time: u32,
) -> Result<(ButtplugServer, Vec<ConformanceDeviceHandle>), ButtplugServerError> {
  // Get the three test devices
  let device_defs = conformance_device_definitions();

  // Build the device configuration manager
  let mut dcm_builder = load_protocol_configs(&None, &None, false)
    .map_err(|e| ButtplugServerError::DeviceConfigurationManagerError(e))?;

  // Register each device's communication specifier and device definition
  for device_def in &device_defs {
    dcm_builder.communication_specifier("conformance", &[device_def.specifier.clone()]);
    dcm_builder.base_device_definition(&device_def.base_identifier, &device_def.definition);
  }

  // Finish the DCM
  let dcm = dcm_builder
    .finish()
    .map_err(|e| ButtplugServerError::DeviceConfigurationManagerError(e))?;

  // Create the conformance device communication manager and add devices
  let mut conformance_dcm_builder = ConformanceDeviceCommunicationManagerBuilder::default();
  let mut device_handles = Vec::new();

  for device_def in &device_defs {
    let handle = conformance_dcm_builder.add_device(
      device_def.name,
      &device_def.address,
      device_def.endpoints.clone(),
      device_def.specifier.clone(),
    );
    device_handles.push(handle);
  }

  // Build the device manager with DCM and conformance communication manager
  let mut device_manager_builder = ServerDeviceManagerBuilder::new(dcm);
  device_manager_builder.comm_manager(conformance_dcm_builder);
  let device_manager = device_manager_builder.finish()?;

  // Build and return the server
  let mut server_builder = ButtplugServerBuilder::new(device_manager);
  server_builder.name("Buttplug Conformance Test Server");
  server_builder.max_ping_time(max_ping_time);
  let server = server_builder.finish()?;

  Ok((server, device_handles))
}
