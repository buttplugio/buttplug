// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::util::{
  ButtplugTestServer,
  TestDeviceChannelHost,
  device_test::connector::build_channel_connector,
};
use buttplug_client::{
  ButtplugClient,
  ButtplugClientDevice,
  ButtplugClientEvent,
  device::{ClientDeviceCommandValue, ClientDeviceFeature, ClientDeviceOutputCommand},
};
use buttplug_client_in_process::ButtplugInProcessClientConnectorBuilder;
use buttplug_core::{message::OutputType, util::async_manager};
use buttplug_server::{ButtplugServer, ButtplugServerBuilder, device::ServerDeviceManagerBuilder};
use buttplug_server_device_config::load_protocol_configs;
use tokio::sync::Notify;

use super::super::{
  super::TestDeviceCommunicationManagerBuilder,
  DeviceTestCase,
  TestClientCommand,
  TestCommand,
};
use buttplug_core::message::{DeviceFeatureOutput, DeviceFeatureOutputLimits};
use futures::StreamExt;
use log::*;
use std::{sync::Arc, time::Duration};

fn from_type_and_value(output_type: OutputType, value: f64) -> ClientDeviceOutputCommand {
  match output_type {
    OutputType::Constrict => ClientDeviceOutputCommand::Constrict(value.into()),
    OutputType::Temperature => ClientDeviceOutputCommand::Temperature(value.into()),
    OutputType::Led => ClientDeviceOutputCommand::Led(value.into()),
    OutputType::Oscillate => ClientDeviceOutputCommand::Oscillate(value.into()),
    OutputType::Position => ClientDeviceOutputCommand::Position(value.into()),
    OutputType::Rotate => ClientDeviceOutputCommand::Rotate(value.into()),
    OutputType::Spray => ClientDeviceOutputCommand::Spray(value.into()),
    OutputType::Vibrate => ClientDeviceOutputCommand::Vibrate(value.into()),
    _ => panic!("Value not translatable, test cannot run"),
  }
}

// Translate ScalarCmd indexes into feature indexes by skipping over any
// features that are not presented as ScalarCmd actuator types
fn get_scalar_index(device: &ButtplugClientDevice, index: u32) -> &u32 {
  let mut offset = 0;
  let mut iter = device.device_features().iter().filter(|f| {
    let fo = f
      .1
      .feature()
      .output()
      .clone()
      .unwrap_or(DeviceFeatureOutput::default());
    fo.vibrate().is_some()
      || fo.oscillate().is_some()
      || fo.constrict().is_some()
      || fo
        .rotate()
        .as_ref()
        .is_some_and(|r| r.step_limit().start() >= &0)
  });
  while let Some((idx, _)) = iter.next() {
    if offset >= index {
      return idx;
    }
    offset += 1;
  }
  return &0;
}

async fn run_test_client_command(command: &TestClientCommand, device: &ButtplugClientDevice) {
  use TestClientCommand::*;
  match command {
    Scalar(msg) => {
      let fut_vec: Vec<_> = msg
        .iter()
        .map(|cmd| {
          let i = get_scalar_index(device, cmd.index());
          let f = device.device_features()[i].clone();
          f.run_output(&from_type_and_value(cmd.actuator_type(), cmd.scalar()))
        })
        .collect();
      futures::future::try_join_all(fut_vec).await.unwrap();
    }
    Vibrate(msg) => {
      let fut_vec: Vec<_> = msg
        .iter()
        .map(|cmd| {
          let vibe_features: Vec<&ClientDeviceFeature> = device
            .device_features()
            .iter()
            .filter(|f| {
              f.1
                .feature()
                .output()
                .as_ref()
                .is_some_and(|x| x.contains(OutputType::Vibrate))
            })
            .map(|(_, x)| x)
            .collect();
          let f = vibe_features[cmd.index() as usize].clone();
          f.run_output(&from_type_and_value(OutputType::Vibrate, cmd.speed()))
        })
        .collect();
      futures::future::try_join_all(fut_vec).await.unwrap();
    }
    Stop => {
      device.stop().await.expect("Stop failed");
    }
    Rotate(msg) => {
      let fut_vec: Vec<_> = msg
        .iter()
        .map(|cmd| {
          let rotate_features: Vec<&ClientDeviceFeature> = device
            .device_features()
            .iter()
            .filter(|f| {
              f.1
                .feature()
                .output()
                .as_ref()
                .is_some_and(|x| x.contains(OutputType::Rotate))
            })
            .map(|(_, x)| x)
            .collect();
          let f = rotate_features[cmd.index() as usize].clone();
          f.run_output(&ClientDeviceOutputCommand::Rotate(ClientDeviceCommandValue::Percent(
            cmd.speed() * if cmd.clockwise() { 1f64 } else { -1f64 },
          )))
        })
        .collect();
      futures::future::try_join_all(fut_vec).await.unwrap();
    }
    Linear(msg) => {
      let fut_vec: Vec<_> = msg
        .iter()
        .map(|cmd| {
          let f = device.device_features()[&cmd.index()].clone();
          f.run_output(&ClientDeviceOutputCommand::HwPositionWithDuration(
            ((cmd.position()
              * f
                .feature()
                .output()
                .as_ref()
                .unwrap()
                .get(OutputType::HwPositionWithDuration)
                .unwrap()
                .step_count() as f64)
              .ceil() as u32).into(),
            cmd.duration(),
          ))
        })
        .collect();
      futures::future::try_join_all(fut_vec).await.unwrap();
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
          let battery_level = device.battery().await.unwrap() as f64 / 100f64;
          assert_eq!(battery_level, expected_power);
        });
      } else {
        assert_eq!(
          device.battery().await.unwrap() as f64 / 100f64,
          *expected_power
        );
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
  let base_cfg = if let Some(device_config_file) = &test_case.device_config_file {
    let config_file_path = std::path::Path::new(
      &std::env::var("CARGO_MANIFEST_DIR").expect("Should have manifest path"),
    )
    .join("tests")
    .join("util")
    .join("device_test")
    .join("device_test_case")
    .join("config")
    .join(device_config_file);

    Some(std::fs::read_to_string(config_file_path).expect("Should be able to load config"))
  } else {
    None
  };
  let user_cfg = if let Some(user_device_config_file) = &test_case.user_device_config_file {
    let config_file_path = std::path::Path::new(
      &std::env::var("CARGO_MANIFEST_DIR").expect("Should have manifest path"),
    )
    .join("tests")
    .join("util")
    .join("device_test")
    .join("device_test_case")
    .join("config")
    .join(user_device_config_file);
    Some(std::fs::read_to_string(config_file_path).expect("Should be able to load config"))
  } else {
    None
  };

  let dcm = load_protocol_configs(&base_cfg, &user_cfg, false)
    .unwrap()
    .finish()
    .unwrap();
  // Create our TestDeviceManager with the device identifier we want to create
  let mut builder = TestDeviceCommunicationManagerBuilder::default();
  let mut device_channels = vec![];
  for device in &test_case.devices {
    info!("identifier: {:?}", device.identifier);
    device_channels.push(builder.add_test_device(&device.identifier));
  }
  let dm = ServerDeviceManagerBuilder::new(dcm)
    .comm_manager(builder)
    .finish()
    .unwrap();

  (
    ButtplugServerBuilder::new(dm)
      .finish()
      .expect("Should always build"),
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
        let device = client.devices().get(device_index).unwrap().clone();
        for message in messages {
          run_test_client_command(message, &device).await;
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
