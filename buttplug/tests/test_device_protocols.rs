mod util;
use std::time::Duration;

use buttplug::{
  client::{ButtplugClient, ButtplugClientEvent, ScalarCommand, LinearCommand},
  core::{
    connector::ButtplugInProcessClientConnectorBuilder,
    messages::{ButtplugDeviceCommandMessageUnion, Endpoint},
  },
  server::{device::hardware::{HardwareCommand, HardwareEvent}, ButtplugServerBuilder},
  util::async_manager
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use util::test_device_manager::{TestDeviceCommunicationManagerBuilder, TestDeviceIdentifier};
use tracing::*;
use test_case::test_case;

#[derive(Serialize, Deserialize)]
struct TestDevice {
  identifier: TestDeviceIdentifier,
  expected_name: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct TestHardwareNotification {
  endpoint: Endpoint,
  data: Vec<u8>
}

#[derive(Serialize, Deserialize)]
enum TestHardwareEvent {
  Notifications(Vec<TestHardwareNotification>),
  Disconnect
}

#[derive(Serialize, Deserialize)]
enum TestCommand {
  Messages {
    device_index: u32,
    messages: Vec<ButtplugDeviceCommandMessageUnion>,
  },
  Commands {
    device_index: u32,
    commands: Vec<HardwareCommand>,
  },
  Events {
    device_index: u32,
    events: Vec<TestHardwareEvent>,
  }
}

#[derive(Serialize, Deserialize)]
struct DeviceTestCase {
  devices: Vec<TestDevice>,
  device_config_file: Option<String>,
  user_device_config_file: Option<String>,
  device_init: Option<Vec<TestCommand>>,
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

  if let Some(device_init) = &test_case.device_init {
    // Parse send message into client calls, receives into response checks
    for command in device_init {
      match command {
        TestCommand::Messages { device_index: _, messages: _ } => {
          panic!("Shouldn't have messages during initialization");
        }
        TestCommand::Commands { device_index, commands } => {
          let device_receiver = &mut device_channels[*device_index as usize].receiver;
          for command in commands {
            tokio::select! {
              _ = tokio::time::sleep(Duration::from_millis(100)) => {
                panic!("Timeout while waiting for device output!")
              }
              event = device_receiver.recv() => {
                info!("Got event {:?}", event);
                if let Some(command_event) = event {
                  assert_eq!(command_event, *command);
                } else {
                  panic!("Should not drop device command receiver");
                }
              }
            }
          }
        }
        TestCommand::Events { device_index, events } => {
          let device_sender = &device_channels[*device_index as usize].sender;
          for event in events {
            match event {
              TestHardwareEvent::Notifications(notifications) => {
                for notification in notifications {
                  device_sender.send(HardwareEvent::Notification(String::new(), notification.endpoint, notification.data.clone())).await.expect("Should always succeed");
                }
              }
              TestHardwareEvent::Disconnect => {
  
              }
            }
          }
        }
      }
    }
  }

  // Scan for devices, wait 'til we get all of the ones we're expecting. Also check names at this
  // point.
  loop {
    tokio::select! {
      _ = tokio::time::sleep(Duration::from_millis(300)) => {
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
      TestCommand::Messages { device_index, messages } => {
        let device = client.devices()[*device_index as usize].clone();
        for message in messages {
          use ButtplugDeviceCommandMessageUnion::*;
          match message {
            ScalarCmd(msg) => {
              // TODO Kinda weird that we're having to rebuild the message.
              device.scalar(&ScalarCommand::ScalarMap(msg.scalars().iter().map(|x| (x.index(), (x.scalar(), x.actuator_type()))).collect())).await.expect("Should always succeed.");
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
      TestCommand::Commands { device_index, commands } => {
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
      TestCommand::Events { device_index, events } => {
        let device_sender = &device_channels[*device_index as usize].sender;
        for event in events {
          match event {
            TestHardwareEvent::Notifications(notifications) => {
              for notification in notifications {
                device_sender.send(HardwareEvent::Notification(String::new(), notification.endpoint, notification.data.clone())).await.expect("Should always succeed");
              }
            }
            TestHardwareEvent::Disconnect => {

            }
          }
        }
      }
    }
  }
}

#[test_case("test_aneros_protocol.yaml" ; "Aneros Protocol")]
#[test_case("test_ankni_protocol.yaml" ; "Ankni Protocol")]
#[test_case("test_cachito_protocol.yaml" ; "Cachito Protocol")]
#[test_case("test_fredorch_protocol.yaml" ; "Fredorch Protocol")]
#[test_case("test_lovense_single_vibrator.yaml" ; "Lovense Protocol - Single Vibrator Device")]
#[test_case("test_lovense_max.yaml" ; "Lovense Protocol - Lovense Max (Constrict)")]
fn test_device_protocols(test_file: &str) {
  async_manager::block_on(async {
    // Load the file list from the test cases directory
    let test_file_path = std::path::Path::new(
      &std::env::var("CARGO_MANIFEST_DIR").expect("Should have manifest path"),
    )
    .join("tests")
    .join("device_test_case")
    .join(test_file);
    // Given the test case object, run the test across all client versions.
    let yaml_test_case =
      std::fs::read_to_string(&test_file_path).expect(&format!("Cannot read file {:?}", test_file_path));
    let test_case =
      serde_yaml::from_str(&yaml_test_case).expect("Could not parse yaml for file.");
    run_test_case(&test_case).await;
  });
}
