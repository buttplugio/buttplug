mod util;

use buttplug_client::{
  ButtplugClient, ButtplugClientEvent,
  device::{ClientDeviceCommandValue, ClientDeviceOutputCommand},
};
use buttplug_client_in_process::ButtplugInProcessClientConnectorBuilder;
use buttplug_core::message::OutputType;
use buttplug_server::device::ServerDeviceManagerBuilder;
use buttplug_server::{
  ButtplugServerBuilder,
  device::hardware::{HardwareCommand, HardwareWriteCmd},
};
use buttplug_server_device_config::load_protocol_configs;
use futures::StreamExt;
use std::time::Duration;
use tokio::time::timeout;
use util::{
  TestDeviceChannelHost, TestDeviceCommunicationManagerBuilder, TestHardwareEvent,
  test_device_manager::TestDeviceIdentifier,
};

async fn recv_command(channel: &mut TestDeviceChannelHost) -> HardwareCommand {
  timeout(Duration::from_millis(500), channel.receiver.recv())
    .await
    .expect("timed out waiting for hardware command")
    .expect("hardware command channel closed")
}

async fn recv_write(channel: &mut TestDeviceChannelHost) -> HardwareWriteCmd {
  loop {
    if let HardwareCommand::Write(cmd) = recv_command(channel).await {
      return cmd;
    }
  }
}

async fn recv_write_with_prefix(
  channel: &mut TestDeviceChannelHost,
  prefix: &[u8],
) -> HardwareWriteCmd {
  loop {
    let cmd = recv_write(channel).await;
    if cmd.data().starts_with(prefix) {
      return cmd;
    }
  }
}

fn lovense_device_type_event(response: &str) -> TestHardwareEvent {
  let bytes = response
    .as_bytes()
    .iter()
    .map(u8::to_string)
    .collect::<Vec<_>>()
    .join(", ");
  serde_yaml::from_str(&format!(
    "!Notifications\n- endpoint: rx\n  data: [{bytes}]\n"
  ))
  .expect("test notification should deserialize")
}

async fn wait_for_device(client: &ButtplugClient) {
  let mut event_stream = client.event_stream();
  loop {
    match timeout(Duration::from_millis(500), event_stream.next()).await {
      Ok(Some(ButtplugClientEvent::DeviceAdded(_))) => return,
      Ok(Some(_)) => continue,
      Ok(None) => panic!("client event stream closed"),
      Err(_) => panic!("timed out waiting for device"),
    }
  }
}

#[tokio::test]
async fn lovense_solace_pro_stop_cancels_linear_updates() {
  let dcm = load_protocol_configs(&None, &None, false)
    .unwrap()
    .finish()
    .unwrap();

  let mut test_device_builder = TestDeviceCommunicationManagerBuilder::default();
  let mut device_channel = test_device_builder.add_test_device(&TestDeviceIdentifier::new(
    "LVS-DoesntMatter",
    Some("lovense-solace-pro".to_owned()),
  ));

  let dm = ServerDeviceManagerBuilder::new(dcm)
    .comm_manager(test_device_builder)
    .finish()
    .unwrap();
  let server = ButtplugServerBuilder::new(dm).finish().unwrap();

  let client = ButtplugClient::new("Test Client");
  let connector = ButtplugInProcessClientConnectorBuilder::default()
    .server(server)
    .finish();
  client.connect(connector).await.unwrap();
  client.start_scanning().await.unwrap();

  let _subscribe = recv_command(&mut device_channel).await;
  let device_type = recv_write(&mut device_channel).await;
  assert_eq!(device_type.data(), b"DeviceType;");
  device_channel
    .sender
    .send(lovense_device_type_event("BA:253:0082059AD3BD;"))
    .await
    .unwrap();

  wait_for_device(&client).await;
  let device = client.devices().get(&0).expect("device 0").clone();
  let feature = device
    .outputs(OutputType::HwPositionWithDuration)
    .first()
    .expect("Solace Pro should expose position-with-duration")
    .clone();

  feature
    .run_output(&ClientDeviceOutputCommand::HwPositionWithDuration(
      ClientDeviceCommandValue::Steps(100),
      1000,
    ))
    .await
    .unwrap();

  let first_linear_update = recv_write_with_prefix(&mut device_channel, b"FSetSite:").await;
  assert_eq!(first_linear_update.data(), b"FSetSite:10;");

  device.stop().await.unwrap();
  let stop = recv_write_with_prefix(&mut device_channel, b"Mply:").await;
  assert_eq!(stop.data(), b"Mply:0:20;");

  match timeout(Duration::from_millis(250), device_channel.receiver.recv()).await {
    Ok(Some(cmd)) => panic!("received command after stop: {cmd:?}"),
    Ok(None) => panic!("hardware command channel closed"),
    Err(_) => {}
  }
}
