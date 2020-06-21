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
    server::ButtplugServer,
    test::{check_recv_value},
    util::async_manager,
  };
  use futures::StreamExt;

  #[test]
  fn test_version0_connection() {
    let (server, _) = ButtplugServer::new("Test Server", 0);
    let mut serializer = ButtplugServerJSONSerializer::default();
    let rsi = r#"[{"RequestServerInfo":{"Id": 1, "ClientName": "Test Client"}}]"#;
    let output = serializer.deserialize(rsi.to_owned().into()).unwrap();
    async_manager::block_on(async {
      let incoming = server.parse_message(output[0].clone()).await.unwrap();
      let incoming_json = serializer.serialize(vec![incoming]);
      assert_eq!(
        incoming_json,
        format!(
          r#"[{{"ServerInfo":{{"Id":1,"MajorVersion":0,"MinorVersion":0,"BuildVersion":0,"MessageVersion":{},"MaxPingTime":0,"ServerName":"Test Server"}}}}]"#,
          BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION as u32
        ).into()
      );
    });
  }

  #[test]
  fn test_version2_connection() {
    let (server, _) = ButtplugServer::new("Test Server", 0);
    let mut serializer = ButtplugServerJSONSerializer::default();
    let rsi =
      r#"[{"RequestServerInfo":{"Id": 1, "ClientName": "Test Client", "MessageVersion": 2}}]"#;
    let output = serializer.deserialize(rsi.to_owned().into()).unwrap();
    async_manager::block_on(async {
      let incoming = server.parse_message(output[0].clone()).await.unwrap();
      let incoming_json = serializer.serialize(vec![incoming]);
      assert_eq!(
        incoming_json,
        format!(
          r#"[{{"ServerInfo":{{"Id":1,"MessageVersion":{},"MaxPingTime":0,"ServerName":"Test Server"}}}}]"#,
          BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION as u32
        ).into()
      );
    });
  }

  #[test]
  fn test_version0_device_added_device_list() {
    let (mut server, mut recv) = ButtplugServer::new("Test Server", 0);
    let mut serializer = ButtplugServerJSONSerializer::default();

    async_manager::block_on(async {
      let helper = server.add_test_comm_manager();
      helper.add_ble_device("Massage Demo").await;
      let rsi = r#"[{"RequestServerInfo":{"Id": 1, "ClientName": "Test Client"}}]"#;
      let mut output = server
        .parse_message(serializer.deserialize(rsi.to_owned().into()).unwrap()[0].clone())
        .await
        .unwrap();
      assert_eq!(
        serializer.serialize(vec!(output)),
        format!(
          r#"[{{"ServerInfo":{{"Id":1,"MajorVersion":0,"MinorVersion":0,"BuildVersion":0,"MessageVersion":{},"MaxPingTime":0,"ServerName":"Test Server"}}}}]"#,
          BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION as u32
        ).into()
      );
      // Skip JSON parsing here, we aren't converting versions.
      let reply = server
        .parse_message(messages::StartScanning::default().into())
        .await;
      assert!(reply.is_ok(), format!("Should get back ok: {:?}", reply));
      // Check that we got an event back about a new device.
      let msg = recv.next().await.unwrap();
      // We should get back an aneros with only SingleMotorVibrateCmd
      assert_eq!(
        serializer.serialize(vec!(msg)),
        r#"[{"DeviceAdded":{"Id":0,"DeviceIndex":0,"DeviceName":"Aneros Vivi","DeviceMessages":["SingleMotorVibrateCmd"]}}]"#.to_owned().into()
      );
      let rdl = serializer
        .deserialize(ButtplugSerializedMessage::Text(
          r#"[{"RequestDeviceList": { "Id": 1}}]"#.to_owned(),
        ))
        .unwrap();
      output = server.parse_message(rdl[0].clone()).await.unwrap();
      assert_eq!(
        serializer.serialize(vec!(output)),
        r#"[{"DeviceList":{"Id":1,"Devices":[{"DeviceIndex":0,"DeviceName":"Aneros Vivi","DeviceMessages":["SingleMotorVibrateCmd"]}]}}]"#.to_owned().into()
      );
    });
  }

  #[test]
  fn test_version0_singlemotorvibratecmd() {
    let (mut server, mut recv) = ButtplugServer::new("Test Server", 0);
    let mut serializer = ButtplugServerJSONSerializer::default();
    async_manager::block_on(async {
      let helper = server.add_test_comm_manager();
      let device = helper.add_ble_device("Massage Demo").await;

      let rsi = r#"[{"RequestServerInfo":{"Id": 1, "ClientName": "Test Client"}}]"#;
      let output = server
        .parse_message(serializer.deserialize(rsi.to_owned().into()).unwrap()[0].clone())
        .await
        .unwrap();
      assert_eq!(
        serializer.serialize(vec!(output)),
        format!(
          r#"[{{"ServerInfo":{{"Id":1,"MajorVersion":0,"MinorVersion":0,"BuildVersion":0,"MessageVersion":{},"MaxPingTime":0,"ServerName":"Test Server"}}}}]"#,
          BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION as u32
        ).into()
      );
      // Skip JSON parsing here, we aren't converting versions.
      let reply = server
        .parse_message(messages::StartScanning::default().into())
        .await;
      assert!(reply.is_ok(), format!("Should get back ok: {:?}", reply));
      // Check that we got an event back about a new device.
      let msg = recv.next().await.unwrap();
      // We should get back an aneros with only SingleMotorVibrateCmd
      assert_eq!(
        serializer.serialize(vec!(msg)),
        r#"[{"DeviceAdded":{"Id":0,"DeviceIndex":0,"DeviceName":"Aneros Vivi","DeviceMessages":["SingleMotorVibrateCmd"]}}]"#.to_owned().into()
      );
      let output2 = server
        .parse_message(
          serializer
            .deserialize(
              r#"[{"SingleMotorVibrateCmd": { "Id": 2, "DeviceIndex": 0, "Speed": 0.5}}]"#
                .to_owned()
                .into(),
            )
            .unwrap()[0].clone(),
        )
        .await
        .unwrap();
      assert_eq!(
        serializer.serialize(vec!(output2)),
        r#"[{"Ok":{"Id":2}}]"#.to_owned().into()
      );
      let command_receiver = device.get_endpoint_channel(&Endpoint::Tx).unwrap().receiver;
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF1, 63], false)),
      )
      .await;
    });
  }
}
