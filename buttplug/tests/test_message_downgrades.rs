extern crate buttplug;

#[cfg(test)]
mod test {
  use async_std::{prelude::StreamExt, task};
  use buttplug::{
    core::messages::{self, BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION},
    device::{
      device::{DeviceImplCommand, DeviceWriteCmd},
      Endpoint,
    },
    server::wrapper::ButtplugJSONServerWrapper,
    server::ButtplugServerWrapper,
    test::{check_recv_value, TestDevice},
  };

  #[test]
  fn test_version0_connection() {
    let _ = env_logger::builder().is_test(true).try_init();
    task::block_on(async {
      let (mut server, _) = ButtplugJSONServerWrapper::new("Test Server", 0);
      let rsi = r#"[{"RequestServerInfo":{"Id": 1, "ClientName": "Test Client"}}]"#;
      let output = server.parse_message(rsi.to_owned()).await;
      assert_eq!(
        output,
        format!(
          r#"[{{"ServerInfo":{{"Id":1,"MajorVersion":0,"MinorVersion":0,"BuildVersion":0,"MessageVersion":{},"MaxPingTime":0,"ServerName":"Test Server"}}}}]"#,
          BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION as u32
        )
      );
    });
  }

  #[test]
  fn test_version2_connection() {
    let _ = env_logger::builder().is_test(true).try_init();
    task::block_on(async {
      let (mut server, _) = ButtplugJSONServerWrapper::new("Test Server", 0);
      let rsi =
        r#"[{"RequestServerInfo":{"Id": 1, "ClientName": "Test Client", "MessageVersion": 2}}]"#;
      let output = server.parse_message(rsi.to_owned()).await;
      assert_eq!(
        output,
        format!(
          r#"[{{"ServerInfo":{{"Id":1,"MessageVersion":{},"MaxPingTime":0,"ServerName":"Test Server"}}}}]"#,
          BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION as u32
        )
      );
    });
  }

  #[test]
  fn test_version0_device_added_device_list() {
    let _ = env_logger::builder().is_test(true).try_init();
    let (mut server, mut recv) = ButtplugJSONServerWrapper::new("Test Server", 0);
    let (_, device_creator) = TestDevice::new_bluetoothle_test_device_impl_creator("Massage Demo");

    task::block_on(async {
      let devices = server.server_ref().add_test_comm_manager();
      devices.lock().await.push(Box::new(device_creator));
      let rsi = r#"[{"RequestServerInfo":{"Id": 1, "ClientName": "Test Client"}}]"#;
      let mut output = server.parse_message(rsi.to_owned()).await;
      assert_eq!(
        output,
        format!(
          r#"[{{"ServerInfo":{{"Id":1,"MajorVersion":0,"MinorVersion":0,"BuildVersion":0,"MessageVersion":{},"MaxPingTime":0,"ServerName":"Test Server"}}}}]"#,
          BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION as u32
        )
      );
      // Skip JSON parsing here, we aren't converting versions.
      let reply = server
        .server_ref()
        .parse_message(&messages::StartScanning::default().into())
        .await;
      assert!(reply.is_ok(), format!("Should get back ok: {:?}", reply));
      // Check that we got an event back about a new device.
      let msg = recv.next().await.unwrap();
      // We should get back an aneros with only SingleMotorVibrateCmd
      assert_eq!(
        msg,
        r#"[{"DeviceAdded":{"Id":0,"DeviceIndex":0,"DeviceName":"Aneros Vivi","DeviceMessages":["SingleMotorVibrateCmd"]}}]"#
      );
      output = server
        .parse_message(r#"[{"RequestDeviceList": { "Id": 1}}]"#.to_owned())
        .await;
      assert_eq!(
        output,
        r#"[{"DeviceList":{"Id":1,"Devices":[{"DeviceIndex":0,"DeviceName":"Aneros Vivi","DeviceMessages":["SingleMotorVibrateCmd"]}]}}]"#
      );
    });
  }

  #[test]
  fn test_version0_singlemotorvibratecmd() {
    let _ = env_logger::builder().is_test(true).try_init();
    let (mut server, mut recv) = ButtplugJSONServerWrapper::new("Test Server", 0);
    let (device, device_creator) =
      TestDevice::new_bluetoothle_test_device_impl_creator("Massage Demo");
    task::block_on(async {
      let devices = server.server_ref().add_test_comm_manager();
      devices.lock().await.push(Box::new(device_creator));

      let rsi = r#"[{"RequestServerInfo":{"Id": 1, "ClientName": "Test Client"}}]"#;
      let output = server.parse_message(rsi.to_owned()).await;
      assert_eq!(
        output,
        format!(
          r#"[{{"ServerInfo":{{"Id":1,"MajorVersion":0,"MinorVersion":0,"BuildVersion":0,"MessageVersion":{},"MaxPingTime":0,"ServerName":"Test Server"}}}}]"#,
          BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION as u32
        )
      );
      // Skip JSON parsing here, we aren't converting versions.
      let reply = server
        .server_ref()
        .parse_message(&messages::StartScanning::default().into())
        .await;
      assert!(reply.is_ok(), format!("Should get back ok: {:?}", reply));
      // Check that we got an event back about a new device.
      let msg = recv.next().await.unwrap();
      // We should get back an aneros with only SingleMotorVibrateCmd
      assert_eq!(
        msg,
        r#"[{"DeviceAdded":{"Id":0,"DeviceIndex":0,"DeviceName":"Aneros Vivi","DeviceMessages":["SingleMotorVibrateCmd"]}}]"#
      );
      let output2 = server
        .parse_message(
          r#"[{"SingleMotorVibrateCmd": { "Id": 2, "DeviceIndex": 0, "Speed": 0.5}}]"#.to_owned(),
        )
        .await;
      assert_eq!(r#"[{"Ok":{"Id":2}}]"#, output2);
      let (_, command_receiver) = device.get_endpoint_channel_clone(&Endpoint::Tx).await;
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF1, 63], false)),
      )
      .await;
    });
  }
}
