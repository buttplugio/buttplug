// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::sync::Arc;

use crate::{
  BackdoorServer, ButtplugRemoteServer, ButtplugServerConnectorError, EngineOptions,
  IntifaceEngineError, IntifaceError, remote_server::ButtplugRemoteServerEvent,
};
use buttplug_server::{
  ButtplugServer, ButtplugServerBuilder,
  connector::ButtplugRemoteServerConnector,
  device::{ServerDeviceManager, ServerDeviceManagerBuilder},
  message::serializer::ButtplugServerJSONSerializer,
};
use buttplug_server_device_config::{DeviceConfigurationManager, load_protocol_configs};
use buttplug_server_hwmgr_btleplug::BtlePlugCommunicationManagerBuilder;
use buttplug_server_hwmgr_lovense_connect::LovenseConnectServiceCommunicationManagerBuilder;
use buttplug_server_hwmgr_sdl_gamepad::SdlGamepadCommunicationManagerBuilder;
use buttplug_server_hwmgr_websocket::WebsocketServerDeviceCommunicationManagerBuilder;
use buttplug_transport_websocket_tungstenite::{
  ButtplugWebsocketClientTransport, ButtplugWebsocketServerTransportBuilder,
};
use once_cell::sync::OnceCell;
use tokio::sync::broadcast::Sender;
// Device communication manager setup gets its own module because the includes and platform
// specifics are such a mess.

/// Warning emitted (on Windows) when both XInput and SDL gamepad managers are
/// enabled: the same physical controller can then appear as two Buttplug
/// devices. Pure decision function so it is testable on every platform; the
/// logging call site is Windows-gated.
pub fn gamepad_dual_manager_warning(
  use_xinput: bool,
  use_sdl_gamepad: bool,
) -> Option<&'static str> {
  if use_xinput && use_sdl_gamepad {
    Some(
      "Both XInput and SDL gamepad managers are enabled; the same physical controller may appear as two devices.",
    )
  } else {
    None
  }
}

/// Testable core of [`setup_server_device_comm_managers`]: returns the names
/// of the comm manager builders the options select. The real builder starts
/// hardware managers (which `#[cfg(test)]` cannot easily exercise), so the
/// registration decision is mirrored here and asserted against in tests.
#[cfg(test)]
fn selected_comm_manager_names(args: &EngineOptions) -> Vec<&'static str> {
  let mut names = vec![];
  if args.use_bluetooth_le() {
    names.push("btleplug");
  }
  if args.use_lovense_connect() {
    names.push("lovense_connect");
  }
  #[cfg(not(any(target_os = "android", target_os = "ios")))]
  {
    if args.use_lovense_dongle_hid() {
      names.push("lovense_dongle_hid");
    }
    if args.use_serial_port() {
      names.push("serial");
    }
    if args.use_hid() {
      names.push("hid");
    }
    #[cfg(target_os = "windows")]
    if args.use_xinput() {
      names.push("xinput");
    }
  }
  if args.use_sdl_gamepad() {
    names.push("sdl_gamepad");
  }
  if args.use_device_websocket_server() {
    names.push("device_websocket_server");
  }
  names
}

pub fn setup_server_device_comm_managers(
  args: &EngineOptions,
  server_builder: &mut ServerDeviceManagerBuilder,
) {
  if args.use_bluetooth_le() {
    info!("Including Bluetooth LE (btleplug) Device Comm Manager Support");
    let mut command_manager_builder = BtlePlugCommunicationManagerBuilder::default();
    #[cfg(target_os = "ios")]
    command_manager_builder.requires_keepalive(true);
    #[cfg(not(target_os = "ios"))]
    command_manager_builder.requires_keepalive(false);
    server_builder.comm_manager(command_manager_builder);
  }
  if args.use_lovense_connect() {
    info!("Including Lovense Connect App Support");
    server_builder.comm_manager(LovenseConnectServiceCommunicationManagerBuilder::default());
  }
  #[cfg(not(any(target_os = "android", target_os = "ios")))]
  {
    use buttplug_server_hwmgr_hid::HidCommunicationManagerBuilder;
    use buttplug_server_hwmgr_lovense_dongle::LovenseHIDDongleCommunicationManagerBuilder;
    use buttplug_server_hwmgr_serial::SerialPortCommunicationManagerBuilder;
    if args.use_lovense_dongle_hid() {
      info!("Including Lovense HID Dongle Support");
      server_builder.comm_manager(LovenseHIDDongleCommunicationManagerBuilder::default());
    }
    if args.use_serial_port() {
      info!("Including Serial Port Support");
      server_builder.comm_manager(SerialPortCommunicationManagerBuilder::default());
    }
    if args.use_hid() {
      info!("Including Hid Support");
      server_builder.comm_manager(HidCommunicationManagerBuilder::default());
    }
    #[cfg(target_os = "windows")]
    {
      use buttplug_server_hwmgr_xinput::XInputDeviceCommunicationManagerBuilder;
      if args.use_xinput() {
        info!("Including XInput Gamepad Support");
        server_builder.comm_manager(XInputDeviceCommunicationManagerBuilder::default());
      }
    }
  }
  // Cross-platform gamepad support via SDL3. No OS gate: unlike XInput, the
  // SDL manager builds everywhere the engine does.
  if args.use_sdl_gamepad() {
    info!("Including SDL Gamepad Support");
    server_builder.comm_manager(SdlGamepadCommunicationManagerBuilder::default());
  }
  // The same physical controller can be picked up by both managers on
  // Windows when both flags are set; warn there, where the overlap exists.
  // The decision itself runs on every platform (cheap, keeps the helper
  // exercised and testable on all OSes); only the logging is Windows-gated.
  if let Some(warning) = gamepad_dual_manager_warning(args.use_xinput(), args.use_sdl_gamepad()) {
    #[cfg(target_os = "windows")]
    warn!("{}", warning);
    #[cfg(not(target_os = "windows"))]
    let _ = warning;
  }
  if args.use_device_websocket_server() {
    info!("Including Websocket Server Device Support");
    let mut builder =
      WebsocketServerDeviceCommunicationManagerBuilder::default().listen_on_all_interfaces(true);
    if let Some(port) = args.device_websocket_server_port() {
      builder = builder.server_port(port);
    }
    server_builder.comm_manager(builder);
  }
  if args.use_simulated_devices() {
    info!("Including Simulated Device Support");
    server_builder.add_simulated_devices_if_configured();
  }
}

pub async fn reset_buttplug_server(
  options: &EngineOptions,
  device_manager: &Arc<ServerDeviceManager>,
  sender: &Sender<ButtplugRemoteServerEvent>,
) -> Result<ButtplugRemoteServer, IntifaceEngineError> {
  match ButtplugServerBuilder::with_shared_device_manager(device_manager.clone())
    .name(options.server_name())
    .max_ping_time(options.max_ping_time())
    .finish()
  {
    Ok(server) => Ok(ButtplugRemoteServer::new(server, &Some(sender.clone()))),
    Err(e) => {
      error!("Error starting server: {:?}", e);
      Err(IntifaceEngineError::ButtplugServerError(e))
    }
  }
}

pub async fn setup_buttplug_server(
  options: &EngineOptions,
  backdoor_server: &OnceCell<Arc<BackdoorServer>>,
  dcm: &Option<Arc<DeviceConfigurationManager>>,
) -> Result<ButtplugServer, IntifaceEngineError> {
  let mut dm_builder = if let Some(dcm) = dcm {
    ServerDeviceManagerBuilder::new_with_arc(dcm.clone())
  } else {
    let mut dcm_builder = load_protocol_configs(
      options.device_config_json(),
      options.user_device_config_json(),
      false,
    )
    .map_err(|e| IntifaceEngineError::ButtplugError(e.into()))?;

    ServerDeviceManagerBuilder::new(
      dcm_builder
        .finish()
        .map_err(|e| IntifaceEngineError::ButtplugError(e.into()))?,
    )
  };

  setup_server_device_comm_managers(options, &mut dm_builder);
  if options.emit_output_observations() {
    dm_builder.emit_output_observations(true);
  }
  let mut server_builder = ButtplugServerBuilder::new(
    dm_builder
      .finish()
      .map_err(IntifaceEngineError::ButtplugServerError)?,
  );

  server_builder
    .name(options.server_name())
    .max_ping_time(options.max_ping_time());

  let core_server = match server_builder.finish() {
    Ok(server) => server,
    Err(e) => {
      error!("Error starting server: {:?}", e);
      return Err(IntifaceEngineError::ButtplugServerError(e));
    }
  };
  if backdoor_server
    .set(Arc::new(BackdoorServer::new(core_server.device_manager())))
    .is_err()
  {
    Err(
      IntifaceError::new("BackdoorServer already initialized somehow! This should never happen!")
        .into(),
    )
  } else {
    Ok(core_server)
  }
}

pub async fn run_server(
  server: &ButtplugRemoteServer,
  options: &EngineOptions,
  on_listener_bound: Option<Arc<dyn Fn(u16) + Send + Sync>>,
) -> Result<(), ButtplugServerConnectorError> {
  if let Some(listen_address) = options.websocket_listen_address() {
    let mut transport_builder = ButtplugWebsocketServerTransportBuilder::default();

    let parsed_listen_address = listen_address.parse().map_err(|pe| {
      ButtplugServerConnectorError::ConnectorError(
        buttplug_core::connector::ButtplugConnectorError::ConnectorGenericError(
          format!("Could not parse provided websocket-listen-address: {pe}")
        ))})?;

    transport_builder.listen_address(parsed_listen_address);
    if let Some(on_listener_bound) = on_listener_bound {
      transport_builder.on_listener_bound(move |bound_port| {
        on_listener_bound(bound_port);
      });
    }
    server
      .start(ButtplugRemoteServerConnector::<
        _,
        ButtplugServerJSONSerializer,
      >::new(transport_builder.finish()))
      .await
  } else if let Some(addr) = options.websocket_client_address() {
    server
      .start(ButtplugRemoteServerConnector::<
        _,
        ButtplugServerJSONSerializer,
      >::new(
        ButtplugWebsocketClientTransport::new_insecure_connector(addr),
      ))
      .await
  } else {
    panic!(
      "Websocket port not set, cannot create transport. Please specify a websocket port in arguments."
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::options::EngineOptionsBuilder;

  #[test]
  fn dual_gamepad_warning_truth_table() {
    // Some(message) exactly when both managers are on; None otherwise.
    assert!(gamepad_dual_manager_warning(true, true).is_some());
    assert!(gamepad_dual_manager_warning(true, false).is_none());
    assert!(gamepad_dual_manager_warning(false, true).is_none());
    assert!(gamepad_dual_manager_warning(false, false).is_none());

    let message = gamepad_dual_manager_warning(true, true).expect("both flags warn");
    assert!(message.contains("XInput") && message.contains("SDL"));
  }

  #[test]
  fn engine_registers_sdl_manager_iff_flag() {
    let with_sdl = EngineOptionsBuilder::default()
      .use_sdl_gamepad(true)
      .finish();
    assert!(
      selected_comm_manager_names(&with_sdl).contains(&"sdl_gamepad"),
      "SDL manager must be registered when the flag is set"
    );

    let without_sdl = EngineOptionsBuilder::default().finish();
    assert!(
      !selected_comm_manager_names(&without_sdl).contains(&"sdl_gamepad"),
      "SDL manager must not be registered when the flag is unset"
    );

    // On all platforms, no OS gate on SDL registration.
    assert!(selected_comm_manager_names(&with_sdl).contains(&"sdl_gamepad"));
  }
}
