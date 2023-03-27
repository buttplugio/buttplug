use crate::util::{
  device_test::connector::build_channel_connector,
  ButtplugTestServer,
  TestDeviceChannelHost,
};
use buttplug::{
  client::{
    ButtplugClient,
    ButtplugClientDevice,
    ButtplugClientEvent,
    LinearCommand,
    RotateCommand,
    ScalarCommand,
    ScalarValueCommand,
  },
  core::connector::ButtplugInProcessClientConnectorBuilder,
  server::{ButtplugServer, ButtplugServerBuilder},
  util::async_manager,
};
use tokio::sync::Notify;

use super::super::{
  super::TestDeviceCommunicationManagerBuilder,
  DeviceTestCase,
  TestClientCommand,
  TestCommand,
};
use futures::StreamExt;
use std::{sync::Arc, time::Duration};
use tracing::*;

async fn run_test_client_command(command: &TestClientCommand, device: &Arc<ButtplugClientDevice>) {
  use TestClientCommand::*;
  match command {
    Scalar(msg) => {
      device
        .scalar(&ScalarCommand::ScalarMap(
          msg
            .iter()
            .map(|x| (x.index(), (x.scalar(), x.actuator_type())))
            .collect(),
        ))
        .await
        .expect("Should always succeed.");
    }
    Vibrate(msg) => {
      device
        .vibrate(&ScalarValueCommand::ScalarValueMap(
          msg.iter().map(|x| (x.index(), x.speed())).collect(),
        ))
        .await
        .expect("Should always succeed.");
    }
    Stop => {
      device.stop().await.expect("Stop failed");
    }
    Rotate(msg) => {
      device
        .rotate(&RotateCommand::RotateMap(
          msg
            .iter()
            .map(|x| (x.index(), (x.speed(), x.clockwise())))
            .collect(),
        ))
        .await
        .expect("Should always succeed.");
    }
    Linear(msg) => {
      device
        .linear(&LinearCommand::LinearVec(
          msg.iter().map(|x| (x.duration(), x.position())).collect(),
        ))
        .await
        .expect("Should always succeed.");
    }
    Battery {
      expected_power,
      run_async,
    } => {
      if *run_async {
        // This is a special case specifically for lovense, since they read their battery off of
        // their notification endpoint. This is a mess but it does the job.
        let device = device.clone();
        let expected_power = *expected_power;
        async_manager::spawn(async move {
          let battery_level = device.battery_level().await.unwrap();
          assert_eq!(battery_level, expected_power);
        });
      } else {
        assert_eq!(device.battery_level().await.unwrap(), *expected_power);
      }
    }
    _ => {
      panic!(
        "Tried to run unhandled TestClientCommand type {:?}",
        command
      );
    }
  }
}

fn build_server(test_case: &DeviceTestCase) -> (ButtplugServer, Vec<TestDeviceChannelHost>) {
  // Create our TestDeviceManager with the device identifier we want to create
  let mut builder = TestDeviceCommunicationManagerBuilder::default();
  let mut device_channels = vec![];
  for device in &test_case.devices {
    info!("identifier: {:?}", device.identifier);
    device_channels.push(builder.add_test_device(&device.identifier));
  }

  // Bring up a server with the TDM
  let mut server_builder = ButtplugServerBuilder::default();
  server_builder.comm_manager(builder);

  if let Some(device_config_file) = &test_case.device_config_file {
    let config_file_path = std::path::Path::new(
      &std::env::var("CARGO_MANIFEST_DIR").expect("Should have manifest path"),
    )
    .join("tests")
    .join("util")
    .join("device_test")
    .join("device_test_case")
    .join("config")
    .join(device_config_file);

    server_builder.device_configuration_json(Some(
      std::fs::read_to_string(config_file_path).expect("Should be able to load config"),
    ));
  }
  if let Some(user_device_config_file) = &test_case.user_device_config_file {
    let config_file_path = std::path::Path::new(
      &std::env::var("CARGO_MANIFEST_DIR").expect("Should have manifest path"),
    )
    .join("tests")
    .join("util")
    .join("device_test")
    .join("device_test_case")
    .join("config")
    .join(user_device_config_file);
    server_builder.user_device_configuration_json(Some(
      std::fs::read_to_string(config_file_path).expect("Should be able to load config"),
    ));
  }
  (
    server_builder.finish().expect("Should always build"),
    device_channels,
  )
}

pub async fn run_embedded_test_case(test_case: &DeviceTestCase) {
  let (server, device_channels) = build_server(test_case);
  // Connect client
  let client = ButtplugClient::new("Test Client");
  let mut in_process_connector_builder = ButtplugInProcessClientConnectorBuilder::default();
  in_process_connector_builder.server(server);
  client
    .connect(in_process_connector_builder.finish())
    .await
    .expect("Test client couldn't connect to embedded process");
  run_test_case(client, device_channels, test_case).await;
}

pub async fn run_json_test_case(test_case: &DeviceTestCase) {
  let notify = Arc::new(Notify::default());

  let (client_connector, server_connector) = build_channel_connector(&notify);

  let (server, device_channels) = build_server(test_case);
  let remote_server = ButtplugTestServer::new(server);
  async_manager::spawn(async move {
    remote_server
      .start(server_connector)
      .await
      .expect("Should always succeed");
  });

  // Connect client
  let client = ButtplugClient::new("Test Client");
  client
    .connect(client_connector)
    .await
    .expect("Test client couldn't connect to embedded process");
  run_test_case(client, device_channels, test_case).await;
}

pub async fn run_test_case(
  client: ButtplugClient,
  mut device_channels: Vec<TestDeviceChannelHost>,
  test_case: &DeviceTestCase,
) {
  let mut event_stream = client.event_stream();

  client
    .start_scanning()
    .await
    .expect("Scanning should work.");

  if let Some(device_init) = &test_case.device_init {
    // Parse send message into client calls, receives into response checks
    for command in device_init {
      match command {
        TestCommand::Messages {
          device_index: _,
          messages: _,
        } => {
          panic!("Shouldn't have messages during initialization");
        }
        TestCommand::Commands {
          device_index,
          commands,
        } => {
          let device_receiver = &mut device_channels[*device_index as usize].receiver;
          for command in commands {
            tokio::select! {
              _ = tokio::time::sleep(Duration::from_millis(500)) => {
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
        TestCommand::Events {
          device_index,
          events,
        } => {
          let device_sender = &device_channels[*device_index as usize].sender;
          for event in events {
            device_sender.send(event.clone()).await.unwrap();
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
          if let Some(expected_display_name) = &test_case.devices[device_added.index() as usize].expected_display_name {
            assert_eq!(Some(expected_display_name.clone()), *device_added.display_name());
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
      TestCommand::Messages {
        device_index,
        messages,
      } => {
        let device = &client.devices()[*device_index as usize];
        for message in messages {
          run_test_client_command(message, device).await;
        }
      }
      TestCommand::Commands {
        device_index,
        commands,
      } => {
        let device_receiver = &mut device_channels[*device_index as usize].receiver;
        for command in commands {
          tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(500)) => {
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
      TestCommand::Events {
        device_index,
        events,
      } => {
        let device_sender = &device_channels[*device_index as usize].sender;
        for event in events {
          device_sender.send(event.clone()).await.unwrap();
        }
      }
    }
  }
}
