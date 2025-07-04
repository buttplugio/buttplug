use std::sync::Arc;

use crate::{
  remote_server::ButtplugRemoteServerEvent, BackdoorServer, ButtplugRemoteServer, ButtplugServerConnectorError, EngineOptions, IntifaceEngineError, IntifaceError
};
use buttplug_server::{
  ButtplugServerBuilder,
  connector::ButtplugRemoteServerConnector,
  device::{ServerDeviceManager, ServerDeviceManagerBuilder},
  message::serializer::ButtplugServerJSONSerializer,
};
use buttplug_server_device_config::{DeviceConfigurationManager, load_protocol_configs};
use buttplug_server_hwmgr_btleplug::BtlePlugCommunicationManagerBuilder;
use buttplug_server_hwmgr_lovense_connect::LovenseConnectServiceCommunicationManagerBuilder;
use buttplug_server_hwmgr_websocket::WebsocketServerDeviceCommunicationManagerBuilder;
use buttplug_transport_websocket_tungstenite::{
  ButtplugWebsocketClientTransport, ButtplugWebsocketServerTransportBuilder,
};
use once_cell::sync::OnceCell;
use tokio::sync::broadcast::Sender;
// Device communication manager setup gets its own module because the includes and platform
// specifics are such a mess.

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
  if args.use_device_websocket_server() {
    info!("Including Websocket Server Device Support");
    let mut builder =
      WebsocketServerDeviceCommunicationManagerBuilder::default().listen_on_all_interfaces(true);
    if let Some(port) = args.device_websocket_server_port() {
      builder = builder.server_port(port);
    }
    server_builder.comm_manager(builder);
  }
}

pub async fn reset_buttplug_server(
  options: &EngineOptions,
  device_manager: &Arc<ServerDeviceManager>,
  sender: &Sender<ButtplugRemoteServerEvent>
) -> Result<ButtplugRemoteServer, IntifaceEngineError> {
  match ButtplugServerBuilder::with_shared_device_manager(device_manager.clone())
    .name(options.server_name())
    .max_ping_time(options.max_ping_time())
    .finish()
  {
    Ok(server) => Ok(ButtplugRemoteServer::new(server, &Some(sender.clone()))),
    Err(e) => {
      error!("Error starting server: {:?}", e);
      return Err(IntifaceEngineError::ButtplugServerError(e));
    }
  }
}

pub async fn setup_buttplug_server(
  options: &EngineOptions,
  backdoor_server: &OnceCell<Arc<BackdoorServer>>,
  dcm: &Option<Arc<DeviceConfigurationManager>>,
) -> Result<ButtplugRemoteServer, IntifaceEngineError> {
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
  let mut server_builder = ButtplugServerBuilder::new(
    dm_builder
      .finish()
      .map_err(|e| IntifaceEngineError::ButtplugServerError(e))?,
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
    Ok(ButtplugRemoteServer::new(core_server, &None))
  }
}

pub async fn run_server(
  server: &ButtplugRemoteServer,
  options: &EngineOptions,
) -> Result<(), ButtplugServerConnectorError> {
  if let Some(port) = options.websocket_port() {
    server
      .start(ButtplugRemoteServerConnector::<
        _,
        ButtplugServerJSONSerializer,
      >::new(
        ButtplugWebsocketServerTransportBuilder::default()
          .port(port)
          .listen_on_all_interfaces(options.websocket_use_all_interfaces())
          .finish(),
      ))
      .await
  } else if let Some(addr) = options.websocket_client_address() {
    server
      .start(ButtplugRemoteServerConnector::<
        _,
        ButtplugServerJSONSerializer,
      >::new(
        ButtplugWebsocketClientTransport::new_insecure_connector(&addr),
      ))
      .await
  } else {
    panic!(
      "Websocket port not set, cannot create transport. Please specify a websocket port in arguments."
    );
  }
}
