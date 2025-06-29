// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::ButtplugInProcessClientConnectorBuilder;
use buttplug_client::ButtplugClient;
use buttplug_server::{device::ServerDeviceManagerBuilder, ButtplugServerBuilder};
use buttplug_server_device_config::DeviceConfigurationManagerBuilder;

/// Convenience function for creating in-process connectors.
///
/// Creates a [ButtplugClient] event loop, with an in-process connector with
/// all device managers that ship with the library and work on the current
/// platform added to it already. Takes a maximum ping time to build the
/// server with, other parameters match `run()`.
///
/// # When To Use This Instead of `run()`
///
/// If you just want to build a quick example and save yourself a few use
/// statements and setup, this will get you going. For anything *production*,
/// we recommend using `run()` as you will have more control over what
/// happens. This method may gain/lose device comm managers at any time.
///
/// # The Device I Want To Use Doesn't Show Up
///
/// If you are trying to use this method to create your client, and do not see
/// the devices you want, there are a couple of things to check:
///
/// - Are you on a platform that the device communication manager supports?
///   For instance, we only support XInput on windows.
/// - Did the developers add a new Device CommunicationManager type and forget
///   to add it to this method? _It's more likely than you think!_ [File a
///   bug](https://github.com/buttplugio/buttplug-rs/issues).
///
/// # Errors
///
/// If the library was compiled without any device managers, the
/// [ButtplugClient] will have nothing to do. This is considered a
/// catastrophic failure and the library will return an error.
///
/// If the library is using outside device managers, it is recommended to
/// build your own connector, add your device manager to those, and use the
/// `run()` method to pass it in.
pub async fn in_process_client(client_name: &str) -> ButtplugClient {
  let dcm = DeviceConfigurationManagerBuilder::default()
    .finish()
    .unwrap();

  let mut device_manager_builder = ServerDeviceManagerBuilder::new(dcm);
  #[cfg(feature = "btleplug-manager")]
  {
    use buttplug_server_hwmgr_btleplug::BtlePlugCommunicationManagerBuilder;
    device_manager_builder.comm_manager(BtlePlugCommunicationManagerBuilder::default());
  }
  #[cfg(feature = "websocket-manager")]
  {
    use buttplug_server_hwmgr_websocket::WebsocketServerDeviceCommunicationManagerBuilder;
    device_manager_builder.comm_manager(
      WebsocketServerDeviceCommunicationManagerBuilder::default().listen_on_all_interfaces(true),
    );
  }
  #[cfg(all(
    feature = "serial-manager",
    any(target_os = "windows", target_os = "macos", target_os = "linux")
  ))]
  {
    use buttplug_server_hwmgr_serial::SerialPortCommunicationManagerBuilder;
    device_manager_builder.comm_manager(SerialPortCommunicationManagerBuilder::default());
  }
  #[cfg(feature = "lovense-connect-service-manager")]
  {
    use buttplug_server_hwmgr_lovense_connect::LovenseConnectServiceCommunicationManagerBuilder;
    device_manager_builder
      .comm_manager(LovenseConnectServiceCommunicationManagerBuilder::default());
  }
  #[cfg(all(
    feature = "lovense-dongle-manager",
    any(target_os = "windows", target_os = "macos", target_os = "linux")
  ))]
  {
    use buttplug_server_hwmgr_lovense_dongle::LovenseHIDDongleCommunicationManagerBuilder;
    device_manager_builder.comm_manager(LovenseHIDDongleCommunicationManagerBuilder::default());
  }
  #[cfg(all(feature = "xinput-manager", target_os = "windows"))]
  {
    use buttplug_server_hwmgr_xinput::XInputDeviceCommunicationManagerBuilder;
    device_manager_builder.comm_manager(XInputDeviceCommunicationManagerBuilder::default());
  }
  let server_builder = ButtplugServerBuilder::new(device_manager_builder.finish().unwrap());
  let server = server_builder.finish().unwrap();
  let connector = ButtplugInProcessClientConnectorBuilder::default()
    .server(server)
    .finish();
  let client = ButtplugClient::new(client_name);
  client.connect(connector).await.unwrap();
  client
}
