// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use tracing::Level;

use buttplug_client::device::{ClientDeviceFeature, ClientDeviceOutputCommand};
use buttplug_client::{ButtplugClient, ButtplugClientDevice, ButtplugClientEvent};
use buttplug_client_in_process::ButtplugInProcessClientConnectorBuilder;
use buttplug_core::message::ButtplugDeviceMessageNameV4::OutputCmd;
use buttplug_core::message::{
  DeviceFeature,
  DeviceFeatureOutput,
  OutputCommand,
  OutputType,
  OutputValue,
};
use buttplug_server::ButtplugServerBuilder;
use buttplug_server::device::ServerDeviceManagerBuilder;
use buttplug_server_device_config::load_protocol_configs;
use buttplug_server_hwmgr_btleplug::BtlePlugCommunicationManagerBuilder;
use futures::StreamExt;
use futures::future::try_join;
use log::error;
use std::collections::{HashMap, HashSet};
use std::{fs, sync::Arc, time::Duration};
use tokio::time::sleep;

async fn set_level_and_wait(
  dev: &ButtplugClientDevice,
  feature: &ClientDeviceFeature,
  output_type: &OutputType,
  level: f64,
) {
  let cmd = match output_type {
    OutputType::Vibrate => Ok(ClientDeviceOutputCommand::VibrateFloat(level)),
    OutputType::Rotate => Ok(ClientDeviceOutputCommand::RotateFloat(level)),
    OutputType::Oscillate => Ok(ClientDeviceOutputCommand::OscillateFloat(level)),
    OutputType::Constrict => Ok(ClientDeviceOutputCommand::ConstrictFloat(level)),
    OutputType::Temperature => Ok(ClientDeviceOutputCommand::TemperatureFloat(level)),
    OutputType::Led => Ok(ClientDeviceOutputCommand::LedFloat(level)),
    OutputType::Position => Ok(ClientDeviceOutputCommand::PositionFloat(level)),
    OutputType::Spray => Ok(ClientDeviceOutputCommand::SprayFloat(level)),
    _ => Err(format!("Unknown output type {:?}", output_type)),
  }
  .unwrap();
  feature.send_command(&cmd).await.unwrap();
  println!(
    "{} ({}) Testing feature {}: {}, output {:?} - {}%",
    dev.name(),
    dev.index(),
    feature.feature().feature_index(),
    feature.feature().description(),
    output_type,
    (level * 100.0) as u8
  );
  sleep(Duration::from_secs(1)).await;
}

async fn device_tester() {
  let mut dc = None;
  let mut uc = None;

  dc = None; //Some(fs::read_to_string("C:\\Users\\user\\AppData\\Roaming\\com.nonpolynomial\\intiface_central\\config\\buttplug-device-config-v3.json").expect("Should have been able to read dc"));
  uc = None; //Some(fs::read_to_string("C:\\Users\\user\\AppData\\Roaming\\com.nonpolynomial\\intiface_central\\config\\buttplug-user-device-config-v3.json").expect("Should have been able to read uc"));

  let dcm = load_protocol_configs(&dc, &uc, false)
    .unwrap()
    .finish()
    .unwrap();

  let mut server_builder = ServerDeviceManagerBuilder::new(dcm);
  server_builder.comm_manager(BtlePlugCommunicationManagerBuilder::default());
  //server_builder.comm_manager(LovenseConnectServiceCommunicationManagerBuilder::default());
  //server_builder.comm_manager(LovenseHIDDongleCommunicationManagerBuilder::default());
  //server_builder.comm_manager(LovenseSerialDongleCommunicationManagerBuilder::default());
  //server_builder.comm_manager(WebsocketServerDeviceCommunicationManagerBuilder::default());
  //server_builder.comm_manager(HidCommunicationManagerBuilder::default());
  //server_builder.comm_manager(SerialPortCommunicationManagerBuilder::default());

  let sb = ButtplugServerBuilder::new(server_builder.finish().unwrap());
  let server = sb.finish().unwrap();
  let connector = ButtplugInProcessClientConnectorBuilder::default()
    .server(server)
    .finish();
  let client = ButtplugClient::new("device-tester");
  client.connect(connector).await.unwrap();

  let mut event_stream = client.event_stream();

  // We'll mostly be doing the same thing we did in example #3, up until we get
  // a device.
  if let Err(err) = client.start_scanning().await {
    println!("Client errored when starting scan! {}", err);
    return;
  }

  let exercise_device = |dev: ButtplugClientDevice| async move {
    let mut cmds = vec![];
    dev.device_features().iter().for_each(|(_, feature)| {
      let outs = feature.feature().output().clone().unwrap_or_default();
      if let Some(out) = outs.get(OutputType::Vibrate) {
        cmds.push(feature.vibrate(out.step_count()));
        println!(
          "{} ({}) should start vibrating on feature {}!",
          dev.name(),
          dev.index(),
          feature.feature_index()
        );
      } else if let Some(out) = outs.get(OutputType::Rotate) {
        cmds.push(feature.rotate(out.step_limit().end().to_owned()));
        println!(
          "{} ({}) should start rotating on feature {}!",
          dev.name(),
          dev.index(),
          feature.feature_index()
        );
      } else if let Some(out) = outs.get(OutputType::Oscillate) {
        cmds.push(feature.oscillate(out.step_count()));
        println!(
          "{} ({}) should start oscillating on feature {}!",
          dev.name(),
          dev.index(),
          feature.feature_index()
        );
      } else if let Some(out) = outs.get(OutputType::Constrict) {
        cmds.push(feature.constrict(out.step_count()));
        println!(
          "{} ({}) should start constricting on feature {}!",
          dev.name(),
          dev.index(),
          feature.feature_index()
        );
      } else if let Some(out) = outs.get(OutputType::Temperature) {
        cmds.push(
          feature.send_command(&ClientDeviceOutputCommand::Temperature(
            *out.step_limit().start(),
          )),
        );
        println!(
          "{} ({}) should start heating on feature {}!",
          dev.name(),
          dev.index(),
          feature.feature_index()
        );
      }
    });
    if cmds.len() > 0 {
      // If the device had any features send what used to be scalar commands async,
      // dispatch all commands now in parallel, then go back and stop them in parallel.
      futures::future::join_all(cmds)
        .await
        .iter()
        .for_each(|cmd| {
          if let Err(err) = cmd {
            error!("{:?}", err);
          }
        });

      sleep(Duration::from_secs(5)).await;

      let mut cmds = vec![];
      dev.device_features().iter().for_each(|(_, feature)| {
        let outs = feature.feature().output().clone().unwrap_or_default();
        if outs.get(OutputType::Vibrate).is_some() {
          cmds.push(feature.vibrate(0));
          println!(
            "{} ({}) should stop vibrating on feature {}!",
            dev.name(),
            dev.index(),
            feature.feature_index()
          );
        } else if outs.get(OutputType::Rotate).is_some() {
          cmds.push(feature.rotate(0));
          println!(
            "{} ({}) should stop rotating on feature {}!",
            dev.name(),
            dev.index(),
            feature.feature_index()
          );
        } else if outs.get(OutputType::Oscillate).is_some() {
          cmds.push(feature.oscillate(0));
          println!(
            "{} ({}) should stop oscillating on feature {}!",
            dev.name(),
            dev.index(),
            feature.feature_index()
          );
        } else if outs.get(OutputType::Constrict).is_some() {
          cmds.push(feature.constrict(0));
          println!(
            "{} ({}) should stop constricting on feature {}!",
            dev.name(),
            dev.index(),
            feature.feature_index()
          );
        } else if outs.get(OutputType::Temperature).is_some() {
          cmds.push(feature.send_command(&ClientDeviceOutputCommand::Temperature(0)));
          println!(
            "{} ({}) should stop heating on feature {}!",
            dev.name(),
            dev.index(),
            feature.feature_index()
          );
        }
      });

      futures::future::join_all(cmds)
        .await
        .iter()
        .for_each(|cmd| {
          if let Err(err) = cmd {
            error!("{:?}", err);
          }
        });

      sleep(Duration::from_secs(2)).await;
    }

    // Exercise each feature
    for (_, feature) in dev.device_features() {
      if let Some(out) = feature.feature().output() {
        let outputs: Vec<&OutputType> = [
          OutputType::Constrict,
          OutputType::Temperature,
          OutputType::Led,
          OutputType::Oscillate,
          OutputType::Position,
          OutputType::PositionWithDuration,
          OutputType::Rotate,
          OutputType::Spray,
          OutputType::Unknown,
          OutputType::Vibrate,
        ]
        .iter()
        .filter(|o| out.contains(**o))
        .collect();

        for otype in outputs {
          let output = out.get(*otype).unwrap();
          match otype {
            OutputType::Vibrate
            | OutputType::Constrict
            | OutputType::Oscillate
            | OutputType::Temperature
            | OutputType::Spray
            | OutputType::Led
            | OutputType::Position => {
              set_level_and_wait(&dev, feature, &otype, 0.05).await;
              set_level_and_wait(&dev, feature, &otype, 0.10).await;
              set_level_and_wait(&dev, feature, &otype, 0.25).await;
              set_level_and_wait(&dev, feature, &otype, 0.5).await;
              set_level_and_wait(&dev, feature, &otype, 0.75).await;
              set_level_and_wait(&dev, feature, &otype, 1.0).await;
              set_level_and_wait(&dev, feature, &otype, 0.0).await;
            }
            OutputType::Unknown => {
              error!(
                "{} ({}) Can't test unknown feature {} ({}), output {:?}",
                dev.name(),
                dev.index(),
                feature.feature().feature_index(),
                feature.feature().description(),
                otype
              );
            }
            OutputType::Rotate => {
              if output.step_limit().start() >= &0 {
                set_level_and_wait(&dev, feature, &otype, 0.25).await;
                set_level_and_wait(&dev, feature, &otype, 0.5).await;
                set_level_and_wait(&dev, feature, &otype, 0.75).await;
                set_level_and_wait(&dev, feature, &otype, 1.0).await;
                set_level_and_wait(&dev, feature, &otype, 0.0).await;
              } else {
                set_level_and_wait(&dev, feature, &otype, 0.25).await;
                set_level_and_wait(&dev, feature, &otype, -0.25).await;
                set_level_and_wait(&dev, feature, &otype, 0.5).await;
                set_level_and_wait(&dev, feature, &otype, -0.5).await;
                set_level_and_wait(&dev, feature, &otype, 0.75).await;
                set_level_and_wait(&dev, feature, &otype, -0.75).await;
                set_level_and_wait(&dev, feature, &otype, 1.0).await;
                set_level_and_wait(&dev, feature, &otype, -1.0).await;
                set_level_and_wait(&dev, feature, &otype, 0.0).await;

                set_level_and_wait(&dev, feature, &otype, 0.25).await;
                set_level_and_wait(&dev, feature, &otype, 0.5).await;
                set_level_and_wait(&dev, feature, &otype, 0.75).await;
                set_level_and_wait(&dev, feature, &otype, 1.0).await;
                set_level_and_wait(&dev, feature, &otype, -0.25).await;
                set_level_and_wait(&dev, feature, &otype, -0.5).await;
                set_level_and_wait(&dev, feature, &otype, -0.75).await;
                set_level_and_wait(&dev, feature, &otype, -1.0).await;
                set_level_and_wait(&dev, feature, &otype, 0.0).await;
              }
            }
            OutputType::PositionWithDuration => {
              feature
                .send_command(&ClientDeviceOutputCommand::PositionWithDurationFloat(
                  0.0, 10,
                ))
                .await
                .unwrap();
              println!(
                "{} ({}) Testing feature {}: {}, output {:?} - {}% {}ms",
                dev.name(),
                dev.index(),
                feature.feature().feature_index(),
                feature.feature().description(),
                "PositionWithDuration",
                (0.0 * 100.0) as u8,
                10
              );
              sleep(Duration::from_secs(1)).await;
              feature
                .send_command(&ClientDeviceOutputCommand::PositionWithDurationFloat(
                  0.5, 1000,
                ))
                .await
                .unwrap();
              println!(
                "{} ({}) Testing feature {}: {}, output {:?} - {}% {}ms",
                dev.name(),
                dev.index(),
                feature.feature().feature_index(),
                feature.feature().description(),
                "PositionWithDuration",
                (0.0 * 100.0) as u8,
                1000
              );
              sleep(Duration::from_secs(1)).await;
              feature
                .send_command(&ClientDeviceOutputCommand::PositionWithDurationFloat(
                  0.0, 10,
                ))
                .await
                .unwrap();
              println!(
                "{} ({}) Testing feature {}: {}, output {:?} - {}% {}ms",
                dev.name(),
                dev.index(),
                feature.feature().feature_index(),
                feature.feature().description(),
                "PositionWithDuration",
                (0.0 * 100.0) as u8,
                10
              );
              sleep(Duration::from_secs(1)).await;
              feature
                .send_command(&ClientDeviceOutputCommand::PositionWithDurationFloat(
                  1.0, 500,
                ))
                .await
                .unwrap();
              println!(
                "{} ({}) Testing feature {}: {}, output {:?} - {}% {}ms",
                dev.name(),
                dev.index(),
                feature.feature().feature_index(),
                feature.feature().description(),
                "PositionWithDuration",
                (1.0 * 100.0) as u8,
                500
              );
              sleep(Duration::from_secs(1)).await;
              feature
                .send_command(&ClientDeviceOutputCommand::PositionWithDurationFloat(
                  0.0, 1500,
                ))
                .await
                .unwrap();
              println!(
                "{} ({}) Testing feature {}: {}, output {:?} - {}% {}ms",
                dev.name(),
                dev.index(),
                feature.feature().feature_index(),
                feature.feature().description(),
                "PositionWithDuration",
                (0.0 * 100.0) as u8,
                1500
              );
            }
          }
        }
      }
    }
  };

  loop {
    match event_stream
      .next()
      .await
      .expect("We own the client so the event stream shouldn't die.")
    {
      ButtplugClientEvent::DeviceAdded(dev) => {
        println!("We got a device: {}", dev.name());
        let fut = exercise_device(dev);
        tokio::spawn(async move {
          fut.await;
        });
        // break;
      }
      ButtplugClientEvent::ServerDisconnect => {
        // The server disconnected, which means we're done here, so just
        // break up to the top level.
        println!("Server disconnected!");
        break;
      }
      _ => {
        // Something else happened, like scanning finishing, devices
        // getting removed, etc... Might as well say something about it.
        println!("Got some other kind of event we don't care about");
      }
    }
  }

  // And now we're done!
  println!("Exiting example");
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
  tracing_subscriber::fmt()
    .with_max_level(Level::DEBUG)
    .init();
  device_tester().await;
}
