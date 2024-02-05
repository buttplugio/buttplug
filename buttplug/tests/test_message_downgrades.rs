// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

extern crate buttplug;
mod util;
pub use util::test_device_manager::check_test_recv_value;

use buttplug::{
  core::message::{
    self,
    serializer::{
      ButtplugMessageSerializer,
      ButtplugSerializedMessage,
      ButtplugServerJSONSerializer,
    },
    Endpoint,
  },
  server::{
    device::hardware::{HardwareCommand, HardwareWriteCmd},
    ButtplugServer,
  },
};
use futures::{pin_mut, StreamExt};
use util::test_server_with_device;

#[tokio::test]
async fn test_version0_connection() {
  let server = ButtplugServer::default();
  let serializer = ButtplugServerJSONSerializer::default();
  let rsi = r#"[{"RequestServerInfo":{"Id": 1, "ClientName": "Test Client"}}]"#;
  let output = serializer
    .deserialize(&rsi.to_owned().into())
    .expect("Test, assuming infallible.");
  let incoming = server
    .parse_message(output[0].clone())
    .await
    .expect("Test, assuming infallible.");
  let incoming_json = serializer.serialize(&vec![incoming]);
  assert_eq!(
        incoming_json,
        r#"[{"ServerInfo":{"Id":1,"MajorVersion":0,"MinorVersion":0,"BuildVersion":0,"MessageVersion":0,"MaxPingTime":0,"ServerName":"Buttplug Server"}}]"#.to_owned().into(),
      );
}

#[tokio::test]
async fn test_version2_connection() {
  let server = ButtplugServer::default();
  let serializer = ButtplugServerJSONSerializer::default();
  let rsi =
    r#"[{"RequestServerInfo":{"Id": 1, "ClientName": "Test Client", "MessageVersion": 2}}]"#;
  let output = serializer
    .deserialize(&rsi.to_owned().into())
    .expect("Test, assuming infallible.");
  let incoming = server
    .parse_message(output[0].clone())
    .await
    .expect("Test, assuming infallible.");
  let incoming_json = serializer.serialize(&vec![incoming]);
  assert_eq!(
        incoming_json,
        r#"[{"ServerInfo":{"Id":1,"MessageVersion":2,"MaxPingTime":0,"ServerName":"Buttplug Server"}}]"#.to_owned().into(),
      );
}

#[tokio::test]
async fn test_version0_device_added_device_list() {
  let (server, _) = test_server_with_device("Massage Demo", false).await;
  let recv = server.event_stream();
  pin_mut!(recv);
  let serializer = ButtplugServerJSONSerializer::default();
  let rsi = r#"[{"RequestServerInfo":{"Id": 1, "ClientName": "Test Client"}}]"#;
  let mut output = server
    .parse_message(
      serializer
        .deserialize(&rsi.to_owned().into())
        .expect("Test, assuming infallible.")[0]
        .clone(),
    )
    .await
    .expect("Test, assuming infallible.");
  assert_eq!(
        serializer.serialize(&vec!(output)),
        r#"[{"ServerInfo":{"Id":1,"MajorVersion":0,"MinorVersion":0,"BuildVersion":0,"MessageVersion":0,"MaxPingTime":0,"ServerName":"Buttplug Server"}}]"#.to_owned().into(),
      );
  // Skip JSON parsing here, we aren't converting versions.
  let reply = server
    .parse_message(message::StartScanning::default().into())
    .await;
  assert!(reply.is_ok(), "Should get back ok: {:?}", reply);
  // Check that we got an event back about scanning finishing.
  let mut msg = recv.next().await.expect("Test, assuming infallible.");
  // We should receive ScanningFinished and DeviceAdded, but the order may change.
  let possible_messages: Vec<ButtplugSerializedMessage> = vec![r#"[{"ScanningFinished":{"Id":0}}]"#.to_owned().into(), r#"[{"DeviceAdded":{"Id":0,"DeviceIndex":0,"DeviceName":"Aneros Vivi","DeviceMessages":["SingleMotorVibrateCmd","StopDeviceCmd"]}}]"#.to_owned().into()];
  assert!(possible_messages.contains(&serializer.serialize(&vec!(msg))));
  msg = recv.next().await.expect("Test, assuming infallible.");
  // We should get back an aneros with only SingleMotorVibrateCmd
  assert!(possible_messages.contains(&serializer.serialize(&vec!(msg))));
  let rdl = serializer
    .deserialize(&ButtplugSerializedMessage::Text(
      r#"[{"RequestDeviceList": { "Id": 1}}]"#.to_owned(),
    ))
    .expect("Test, assuming infallible.");
  output = server
    .parse_message(rdl[0].clone())
    .await
    .expect("Test, assuming infallible.");
  assert_eq!(
        serializer.serialize(&vec!(output)),
        r#"[{"DeviceList":{"Id":1,"Devices":[{"DeviceIndex":0,"DeviceName":"Aneros Vivi","DeviceMessages":["SingleMotorVibrateCmd","StopDeviceCmd"]}]}}]"#.to_owned().into()
      );
}

#[tokio::test]
async fn test_version0_singlemotorvibratecmd() {
  let (server, mut device) = test_server_with_device("Massage Demo", false).await;
  let recv = server.event_stream();
  pin_mut!(recv);
  let serializer = ButtplugServerJSONSerializer::default();
  let rsi = r#"[{"RequestServerInfo":{"Id": 1, "ClientName": "Test Client"}}]"#;
  let output = server
    .parse_message(
      serializer
        .deserialize(&rsi.to_owned().into())
        .expect("Test, assuming infallible.")[0]
        .clone(),
    )
    .await
    .expect("Test, assuming infallible.");
  assert_eq!(
        serializer.serialize(&vec!(output)),
        r#"[{"ServerInfo":{"Id":1,"MajorVersion":0,"MinorVersion":0,"BuildVersion":0,"MessageVersion":0,"MaxPingTime":0,"ServerName":"Buttplug Server"}}]"#.to_owned().into(),
      );
  // Skip JSON parsing here, we aren't converting versions.
  let reply = server
    .parse_message(message::StartScanning::default().into())
    .await;
  assert!(reply.is_ok(), "Should get back ok: {:?}", reply);
  // Check that we got an event back about scanning finishing.
  let mut msg = recv.next().await.expect("Test, assuming infallible.");
  // We should receive ScanningFinished and DeviceAdded, but the order may change.
  let possible_messages: Vec<ButtplugSerializedMessage> = vec![r#"[{"ScanningFinished":{"Id":0}}]"#.to_owned().into(), r#"[{"DeviceAdded":{"Id":0,"DeviceIndex":0,"DeviceName":"Aneros Vivi","DeviceMessages":["SingleMotorVibrateCmd","StopDeviceCmd"]}}]"#.to_owned().into()];
  assert!(possible_messages.contains(&serializer.serialize(&vec!(msg))));
  msg = recv.next().await.expect("Test, assuming infallible.");
  // We should get back an aneros with only SingleMotorVibrateCmd
  assert!(possible_messages.contains(&serializer.serialize(&vec!(msg))));
  let output2 = server
    .parse_message(
      serializer
        .deserialize(
          &r#"[{"SingleMotorVibrateCmd": { "Id": 2, "DeviceIndex": 0, "Speed": 0.5}}]"#
            .to_owned()
            .into(),
        )
        .expect("Test, assuming infallible.")[0]
        .clone(),
    )
    .await
    .expect("Test, assuming infallible.");
  assert_eq!(
    serializer.serialize(&vec!(output2)),
    r#"[{"Ok":{"Id":2}}]"#.to_owned().into()
  );
  check_test_recv_value(
    &mut device,
    HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xF1, 64], false)),
  );
}

#[tokio::test]
async fn test_version1_singlemotorvibratecmd() {
  let (server, mut device) = test_server_with_device("Massage Demo", false).await;
  let recv = server.event_stream();
  pin_mut!(recv);
  let serializer = ButtplugServerJSONSerializer::default();
  let rsi =
    r#"[{"RequestServerInfo":{"Id": 1, "ClientName": "Test Client", "MessageVersion": 1}}]"#;
  let output = server
    .parse_message(
      serializer
        .deserialize(&rsi.to_owned().into())
        .expect("Test, assuming infallible.")[0]
        .clone(),
    )
    .await
    .expect("Test, assuming infallible.");
  assert_eq!(
        serializer.serialize(&vec!(output)),
        r#"[{"ServerInfo":{"Id":1,"MajorVersion":0,"MinorVersion":0,"BuildVersion":0,"MessageVersion":1,"MaxPingTime":0,"ServerName":"Buttplug Server"}}]"#.to_owned().into(),
      );
  // Skip JSON parsing here, we aren't converting versions.
  let reply = server
    .parse_message(message::StartScanning::default().into())
    .await;
  assert!(reply.is_ok(), "Should get back ok: {:?}", reply);
  // Check that we got an event back about scanning finishing.
  let mut msg = recv.next().await.expect("Test, assuming infallible.");
  let mut smsg = serializer.serialize(&vec![msg]);
  // We should receive ScanningFinished and DeviceAdded, but the order may change.
  let possible_messages: Vec<ButtplugSerializedMessage> = vec![r#"[{"ScanningFinished":{"Id":0}}]"#.to_owned().into(), r#"[{"DeviceAdded":{"Id":0,"DeviceIndex":0,"DeviceName":"Aneros Vivi","DeviceMessages":{"VibrateCmd":{"FeatureCount":2},"stop_device_cmd":{},"single_motor_vibrate_cmd":{}}}}]"#.to_owned().into()];
  assert!(
    possible_messages.contains(&smsg),
    "We should receive ScanningFinished and DeviceAdded, but the order may change. Got {:?}",
    smsg
  );
  msg = recv.next().await.expect("Test, assuming infallible.");
  smsg = serializer.serialize(&vec![msg]);
  // We should get back an aneros with only SingleMotorVibrateCmd
  assert!(
    possible_messages.contains(&smsg),
    "We should receive ScanningFinished and DeviceAdded, but the order may change. Got {:?}",
    smsg
  );
  let output2 = server
        .parse_message(
            serializer
                .deserialize(
                    &r#"[{"VibrateCmd": { "Id": 2, "DeviceIndex": 0, "Speeds": [{ "Index": 0, "Speed": 0.5 }]}}]"#
                        .to_owned()
                        .into(),
                )
                .expect("Test, assuming infallible.")[0]
                .clone(),
        )
        .await
        .expect("Test, assuming infallible.");
  assert_eq!(
    serializer.serialize(&vec!(output2)),
    r#"[{"Ok":{"Id":2}}]"#.to_owned().into()
  );
  check_test_recv_value(
    &mut device,
    HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xF1, 64], false)),
  );
}

#[tokio::test]
async fn test_version0_oscilatoronly() {
  let (server, mut _device) = test_server_with_device("Xone", false).await;
  let recv = server.event_stream();
  pin_mut!(recv);
  let serializer = ButtplugServerJSONSerializer::default();
  let rsi = r#"[{"RequestServerInfo":{"Id": 1, "ClientName": "Test Client"}}]"#;
  let output = server
    .parse_message(
      serializer
        .deserialize(&rsi.to_owned().into())
        .expect("Test, assuming infallible.")[0]
        .clone(),
    )
    .await
    .expect("Test, assuming infallible.");
  assert_eq!(
        serializer.serialize(&vec!(output)),
        r#"[{"ServerInfo":{"Id":1,"MajorVersion":0,"MinorVersion":0,"BuildVersion":0,"MessageVersion":0,"MaxPingTime":0,"ServerName":"Buttplug Server"}}]"#.to_owned().into(),
      );
  // Skip JSON parsing here, we aren't converting versions.
  let reply = server
    .parse_message(message::StartScanning::default().into())
    .await;
  assert!(reply.is_ok(), "Should get back ok: {:?}", reply);
  // Check that we got an event back about scanning finishing.
  let mut msg = recv.next().await.expect("Test, assuming infallible.");
  let mut smsg = serializer.serialize(&vec![msg]);
  // We should receive ScanningFinished and DeviceAdded, but the order may change.
  let possible_messages: Vec<ButtplugSerializedMessage> = vec![r#"[{"ScanningFinished":{"Id":0}}]"#.to_owned().into(), r#"[{"DeviceAdded":{"Id":0,"DeviceIndex":0,"DeviceName":"MagicMotion Xone","DeviceMessages":["StopDeviceCmd"]}}]"#.to_owned().into()];
  assert!(
    possible_messages.contains(&smsg),
    "We should receive ScanningFinished and DeviceAdded, but the order may change. Got {:?}",
    smsg
  );
  msg = recv.next().await.expect("Test, assuming infallible.");
  smsg = serializer.serialize(&vec![msg]);
  // We should get back an MagicMotion Xone with no actuators
  assert!(
    possible_messages.contains(&smsg),
    "We should receive ScanningFinished and DeviceAdded, but the order may change. Got {:?}",
    smsg
  );
}

#[tokio::test]
async fn test_version1_oscilatoronly() {
  let (server, mut _device) = test_server_with_device("Xone", false).await;
  let recv = server.event_stream();
  pin_mut!(recv);
  let serializer = ButtplugServerJSONSerializer::default();
  let rsi =
    r#"[{"RequestServerInfo":{"Id": 1, "ClientName": "Test Client", "MessageVersion": 1}}]"#;
  let output = server
    .parse_message(
      serializer
        .deserialize(&rsi.to_owned().into())
        .expect("Test, assuming infallible.")[0]
        .clone(),
    )
    .await
    .expect("Test, assuming infallible.");
  assert_eq!(
        serializer.serialize(&vec!(output)),
        r#"[{"ServerInfo":{"Id":1,"MajorVersion":0,"MinorVersion":0,"BuildVersion":0,"MessageVersion":1,"MaxPingTime":0,"ServerName":"Buttplug Server"}}]"#.to_owned().into(),
      );
  // Skip JSON parsing here, we aren't converting versions.
  let reply = server
    .parse_message(message::StartScanning::default().into())
    .await;
  assert!(reply.is_ok(), "Should get back ok: {:?}", reply);
  // Check that we got an event back about scanning finishing.
  let mut msg = recv.next().await.expect("Test, assuming infallible.");
  let mut smsg = serializer.serialize(&vec![msg]);
  // We should receive ScanningFinished and DeviceAdded, but the order may change.
  let possible_messages: Vec<ButtplugSerializedMessage> = vec![r#"[{"ScanningFinished":{"Id":0}}]"#.to_owned().into(), r#"[{"DeviceAdded":{"Id":0,"DeviceIndex":0,"DeviceName":"MagicMotion Xone","DeviceMessages":{"stop_device_cmd":{}}}}]"#.to_owned().into()];
  assert!(
    possible_messages.contains(&smsg),
    "We should receive ScanningFinished and DeviceAdded, but the order may change. Got {:?}",
    smsg
  );
  msg = recv.next().await.expect("Test, assuming infallible.");
  smsg = serializer.serialize(&vec![msg]);
  // We should get back an MagicMotion Xone with no actuators
  assert!(
    possible_messages.contains(&smsg),
    "We should receive ScanningFinished and DeviceAdded, but the order may change. Got {:?}",
    smsg
  );
}
