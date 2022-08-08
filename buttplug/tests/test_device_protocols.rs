mod util;
use std::time::Duration;

use buttplug::{
  client::{ButtplugClient, ButtplugClientEvent, ScalarCommand, LinearCommand},
  core::{
    connector::ButtplugInProcessClientConnectorBuilder,
    messages::{ButtplugDeviceCommandMessageUnion, ScalarCmd, ScalarSubcommand},
  },
  server::{device::hardware::HardwareCommand, ButtplugServerBuilder},
  util::async_manager
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use util::test_device_manager::{TestDeviceCommunicationManagerBuilder, TestDeviceIdentifier};
use tracing::*;

#[derive(Serialize, Deserialize)]
struct TestDevice {
  identifier: TestDeviceIdentifier,
  expected_name: Option<String>,
}

#[derive(Serialize, Deserialize)]
enum TestCommand {
  send {
    device_index: u32,
    messages: Vec<ButtplugDeviceCommandMessageUnion>,
  },
  receive {
    device_index: u32,
    commands: Vec<HardwareCommand>,
  },
}

#[derive(Serialize, Deserialize)]
struct DeviceTestCase {
  devices: Vec<TestDevice>,
  device_config_file: Option<String>,
  user_device_config_file: Option<String>,
  device_commands: Vec<TestCommand>,
}

async fn run_test_case(test_case: &DeviceTestCase) {
  // Create our TestDeviceManager with the device identifier we want to create
  let mut builder = TestDeviceCommunicationManagerBuilder::default();
  let mut device_channels = vec![];
  for device in &test_case.devices {
    device_channels.push(builder.add_test_device(&device.identifier));
  }

  // Bring up a server with the TDM
  let mut server_builder = ButtplugServerBuilder::default();
  server_builder.comm_manager(builder);
  let server = server_builder.finish().expect("Should always build");

  // Connect client
  let client = ButtplugClient::new("Test Client");
  let mut in_process_connector_builder = ButtplugInProcessClientConnectorBuilder::default();
  in_process_connector_builder.server(server);

  let mut event_stream = client.event_stream();

  client.connect(in_process_connector_builder.finish()).await.expect("Test client couldn't connect to embedded process");
  client.start_scanning().await.expect("Scanning should work.");

  // Scan for devices, wait 'til we get all of the ones we're expecting. Also check names at this
  // point.
  loop {
    tokio::select! {
      _ = tokio::time::sleep(Duration::from_millis(100)) => {
        panic!("Timeout while waiting for device scan return!")
      }
      event = event_stream.next() => {
        if let Some(ButtplugClientEvent::DeviceAdded(device_added)) = event {
          // Compare expected device name
          if let Some(expected_name) = &test_case.devices[device_added.index() as usize].expected_name {
            assert_eq!(*expected_name, *device_added.name());
          }
          if client.devices().len() == test_case.devices.len() {
            break;
          }
        } else if event.is_none() {
          panic!("Should not have dropped event stream!");
        } else {
          debug!("Ignoring client message while waiting for devices: {:?}", event);
        }
      }
    }
  }
  
  // Parse send message into client calls, receives into response checks

  for command in &test_case.device_commands {
    match command {
      TestCommand::send { device_index, messages } => {
        let device = client.devices()[*device_index as usize].clone();
        for message in messages {
          use ButtplugDeviceCommandMessageUnion::*;
          match message {
            ScalarCmd(msg) => {
              // TODO Kinda weird that we're having to rebuild the message.
              device.scalar(&ScalarCommand::ScalarVec(msg.scalars().iter().map(|x| (x.scalar(), x.actuator_type())).collect())).await.expect("Should always succeed.");
            }
            StopDeviceCmd(_) => {
              // TODO Kinda weird that we're having to rebuild the message.
              device.stop().await.expect("Stop failed");
            }
            LinearCmd(msg) => {
              device.linear(&LinearCommand::LinearVec(msg.vectors().iter().map(|x| (x.duration(), *x.position())).collect())).await.expect("Should always succeed.")
            }
            _ => {

            }
          }
        }
      }
      TestCommand::receive { device_index, commands } => {
        let device_receiver = &mut device_channels[*device_index as usize].receiver;
        for command in commands {
          tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(100)) => {
              panic!("Timeout while waiting for device output!")
            }
            event = device_receiver.recv() => {
              if let Some(command_event) = event {
                assert_eq!(command_event, *command);
              } else {
                panic!("Should not drop device command receiver");
              }
            }
          }
        }
      }
    }
  }
}

#[test]
fn test_device_protocols() {
  async_manager::block_on(async {
    // Load the file list from the test cases directory
    let test_case_dir = std::path::Path::new(
      &std::env::var("CARGO_MANIFEST_DIR").expect("Should have manifest path"),
    )
    .join(std::path::Path::new("tests"))
    .join(std::path::Path::new("device_test_case"));

    let paths = std::fs::read_dir(&test_case_dir).unwrap();

    for path in paths {
      // Load a test file
      if !path.as_ref().unwrap().metadata().unwrap().is_file() {
        continue;
      }
      // Given the test case object, run the test across all client versions.
      let file_path = path.unwrap().path();
      println!("Running test case {:?}", file_path);
      let yaml_test_case =
        std::fs::read_to_string(&file_path).expect(&format!("Cannot read file {:?}", file_path));
      let test_case =
        serde_yaml::from_str(&yaml_test_case).expect("Could not parse yaml for file.");
      run_test_case(&test_case).await;
    }
  });
}
