extern crate buttplug;

#[cfg(test)]
mod test {
  use buttplug::{
    core::messages::{
      self,
      serializer::{
        ButtplugMessageSerializer,
        ButtplugSerializedMessage,
        ButtplugServerJSONSerializer,
      },
      BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION,
    },
    device::{DeviceImplCommand, DeviceWriteCmd, Endpoint},
    server::comm_managers::test::{check_test_recv_value, TestDeviceCommunicationManagerBuilder},
    server::ButtplugServer,
    util::async_manager,
  };
  use futures::{pin_mut, StreamExt};

  #[test]
  fn test_version0_connection() {
    async_manager::block_on(async {
      let server = ButtplugServer::default();
      let serializer = ButtplugServerJSONSerializer::default();
      let rsi = r#"[{"RequestServerInfo":{"Id": 1, "ClientName": "Test Client"}}]"#;
      let output = serializer
        .deserialize(rsi.to_owned().into())
        .expect("Test, assuming infallible.");
      let incoming = server
        .parse_message(output[0].clone())
        .await
        .expect("Test, assuming infallible.");
      let incoming_json = serializer.serialize(vec![incoming]);
      assert_eq!(
        incoming_json,
        format!(
          r#"[{{"ServerInfo":{{"Id":1,"MajorVersion":0,"MinorVersion":0,"BuildVersion":0,"MessageVersion":{},"MaxPingTime":0,"ServerName":"Buttplug Server"}}}}]"#,
          BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION as u32
        ).into()
      );
    });
  }

  #[test]
  fn test_version2_connection() {
    async_manager::block_on(async {
      let server = ButtplugServer::default();
      let serializer = ButtplugServerJSONSerializer::default();
      let rsi =
        r#"[{"RequestServerInfo":{"Id": 1, "ClientName": "Test Client", "MessageVersion": 2}}]"#;
      let output = serializer
        .deserialize(rsi.to_owned().into())
        .expect("Test, assuming infallible.");
      let incoming = server
        .parse_message(output[0].clone())
        .await
        .expect("Test, assuming infallible.");
      let incoming_json = serializer.serialize(vec![incoming]);
      assert_eq!(
        incoming_json,
        format!(
          r#"[{{"ServerInfo":{{"Id":1,"MessageVersion":{},"MaxPingTime":0,"ServerName":"Buttplug Server"}}}}]"#,
          BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION as u32
        ).into()
      );
    });
  }

  #[test]
  fn test_version0_device_added_device_list() {
    async_manager::block_on(async {
      let server = ButtplugServer::default();
      let recv = server.event_stream();
      pin_mut!(recv);
      let serializer = ButtplugServerJSONSerializer::default();
      let builder = TestDeviceCommunicationManagerBuilder::default();
      let helper = builder.helper();
      server
        .device_manager()
        .add_comm_manager(builder)
        .expect("Test, assuming infallible.");
      helper.add_ble_device("Massage Demo").await;
      let rsi = r#"[{"RequestServerInfo":{"Id": 1, "ClientName": "Test Client"}}]"#;
      let mut output = server
        .parse_message(
          serializer
            .deserialize(rsi.to_owned().into())
            .expect("Test, assuming infallible.")[0]
            .clone(),
        )
        .await
        .expect("Test, assuming infallible.");
      assert_eq!(
        serializer.serialize(vec!(output)),
        format!(
          r#"[{{"ServerInfo":{{"Id":1,"MajorVersion":0,"MinorVersion":0,"BuildVersion":0,"MessageVersion":{},"MaxPingTime":0,"ServerName":"Buttplug Server"}}}}]"#,
          BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION as u32
        ).into()
      );
      // Skip JSON parsing here, we aren't converting versions.
      let reply = server
        .parse_message(messages::StartScanning::default().into())
        .await;
      assert!(reply.is_ok(), "Should get back ok: {:?}", reply);
      // Check that we got an event back about scanning finishing.
      let mut msg = recv.next().await.expect("Test, assuming infallible.");
      // We should receive ScanningFinished and DeviceAdded, but the order may change.
      let possible_messages: Vec<ButtplugSerializedMessage> = vec![r#"[{"ScanningFinished":{"Id":0}}]"#.to_owned().into(), r#"[{"DeviceAdded":{"Id":0,"DeviceIndex":0,"DeviceName":"Aneros Vivi","DeviceMessages":["SingleMotorVibrateCmd","StopDeviceCmd"]}}]"#.to_owned().into()];
      assert!(possible_messages.contains(&serializer.serialize(vec!(msg))));
      msg = recv.next().await.expect("Test, assuming infallible.");
      // We should get back an aneros with only SingleMotorVibrateCmd
      assert!(possible_messages.contains(&serializer.serialize(vec!(msg))));
      let rdl = serializer
        .deserialize(ButtplugSerializedMessage::Text(
          r#"[{"RequestDeviceList": { "Id": 1}}]"#.to_owned(),
        ))
        .expect("Test, assuming infallible.");
      output = server
        .parse_message(rdl[0].clone())
        .await
        .expect("Test, assuming infallible.");
      assert_eq!(
        serializer.serialize(vec!(output)),
        r#"[{"DeviceList":{"Id":1,"Devices":[{"DeviceIndex":0,"DeviceName":"Aneros Vivi","DeviceMessages":["SingleMotorVibrateCmd","StopDeviceCmd"]}]}}]"#.to_owned().into()
      );
    });
  }

  #[test]
  fn test_version0_singlemotorvibratecmd() {
    async_manager::block_on(async {
      let server = ButtplugServer::default();
      let recv = server.event_stream();
      pin_mut!(recv);
      let serializer = ButtplugServerJSONSerializer::default();
      let builder = TestDeviceCommunicationManagerBuilder::default();
      let helper = builder.helper();
      server
        .device_manager()
        .add_comm_manager(builder)
        .expect("Test, assuming infallible.");
      let device = helper.add_ble_device("Massage Demo").await;

      let rsi = r#"[{"RequestServerInfo":{"Id": 1, "ClientName": "Test Client"}}]"#;
      let output = server
        .parse_message(
          serializer
            .deserialize(rsi.to_owned().into())
            .expect("Test, assuming infallible.")[0]
            .clone(),
        )
        .await
        .expect("Test, assuming infallible.");
      assert_eq!(
        serializer.serialize(vec!(output)),
        format!(
          r#"[{{"ServerInfo":{{"Id":1,"MajorVersion":0,"MinorVersion":0,"BuildVersion":0,"MessageVersion":{},"MaxPingTime":0,"ServerName":"Buttplug Server"}}}}]"#,
          BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION as u32
        ).into()
      );
      // Skip JSON parsing here, we aren't converting versions.
      let reply = server
        .parse_message(messages::StartScanning::default().into())
        .await;
      assert!(reply.is_ok(), "Should get back ok: {:?}", reply);
      // Check that we got an event back about scanning finishing.
      let mut msg = recv.next().await.expect("Test, assuming infallible.");
      // We should receive ScanningFinished and DeviceAdded, but the order may change.
      let possible_messages: Vec<ButtplugSerializedMessage> = vec![r#"[{"ScanningFinished":{"Id":0}}]"#.to_owned().into(), r#"[{"DeviceAdded":{"Id":0,"DeviceIndex":0,"DeviceName":"Aneros Vivi","DeviceMessages":["SingleMotorVibrateCmd","StopDeviceCmd"]}}]"#.to_owned().into()];
      assert!(possible_messages.contains(&serializer.serialize(vec!(msg))));
      msg = recv.next().await.expect("Test, assuming infallible.");
      // We should get back an aneros with only SingleMotorVibrateCmd
      assert!(possible_messages.contains(&serializer.serialize(vec!(msg))));
      let output2 = server
        .parse_message(
          serializer
            .deserialize(
              r#"[{"SingleMotorVibrateCmd": { "Id": 2, "DeviceIndex": 0, "Speed": 0.5}}]"#
                .to_owned()
                .into(),
            )
            .expect("Test, assuming infallible.")[0]
            .clone(),
        )
        .await
        .expect("Test, assuming infallible.");
      assert_eq!(
        serializer.serialize(vec!(output2)),
        r#"[{"Ok":{"Id":2}}]"#.to_owned().into()
      );
      let command_receiver = device
        .get_endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible.");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF1, 64], false)),
      );
    });
  }
}
