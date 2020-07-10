  use buttplug::{
    core::{
      errors::ButtplugError,
      messages::{ButtplugMessageSpecVersion, self, ButtplugServerMessage, BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION},
    },
    device::{DeviceImplCommand, DeviceWriteCmd, Endpoint},
    server::{ButtplugServer},
    test::check_recv_value,
    util::async_manager,
  };
  use futures::StreamExt;
  use futures_timer::Delay;
  use async_channel::Receiver;
  use std::time::Duration;
  
  async fn test_server_setup(
    msg_union: messages::ButtplugClientMessage,
  ) -> (ButtplugServer, Receiver<ButtplugServerMessage>) {
    let (server, recv) = ButtplugServer::new("Test Server", 0);
    // assert_eq!(server.server_name, "Test Server");
    match server.parse_message(msg_union).await.unwrap() {
      ButtplugServerMessage::ServerInfo(_s) => assert_eq!(
        _s,
        messages::ServerInfo::new("Test Server", ButtplugMessageSpecVersion::Version2, 0)
      ),
      _ => panic!("Should've received ok"),
    }
    (server, recv)
  }

  #[test]
  fn test_server_handshake() {
    let msg =
      messages::RequestServerInfo::new("Test Client", ButtplugMessageSpecVersion::Version2).into();
    async_manager::block_on(async {
      let (server, _recv) = test_server_setup(msg).await;
      assert!(server.connected());
    });
  }

  #[test]
  fn test_server_version_lt() {
    let msg =
      messages::RequestServerInfo::new("Test Client", ButtplugMessageSpecVersion::Version2).into();
      async_manager::block_on(async {
      test_server_setup(msg).await;
    });
  }

  // TODO Now that we're moving to a spec version enum, this test is invalid
  // because we can't just pass a u8 in. This should be rebuilt using the
  // JSON parser, and it should fail to deserialize the message.
  #[test]
  #[ignore]
  fn test_server_version_gt() {
    let (server, _) = ButtplugServer::new("Test Server", 0);
    let msg =
      messages::RequestServerInfo::new("Test Client", ButtplugMessageSpecVersion::Version2).into();
      async_manager::block_on(async {
      assert!(
        server.parse_message(msg).await.is_err(),
        "Client having higher version than server should fail"
      );
    });
  }

  #[test]
  fn test_ping_timeout() {
    let (server, mut recv) = ButtplugServer::new("Test Server", 100);
    async_manager::block_on(async {
      let msg =
        messages::RequestServerInfo::new("Test Client", BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION);
      Delay::new(Duration::from_millis(150)).await;
      let reply = server.parse_message(msg.into()).await;
      assert!(
        reply.is_ok(),
        format!(
          "ping timer shouldn't start until handshake finished. {:?}",
          reply
        )
      );
      Delay::new(Duration::from_millis(300)).await;
      let pingmsg = messages::Ping::default();
      match server.parse_message(pingmsg.into()).await {
        Ok(_) => panic!("Should get a ping error back!"),
        Err(e) => {
          if let ButtplugError::ButtplugPingError(_) = e {
            // do nothing
          } else {
            panic!("Got wrong type of error back! {:?}", e);
          }
        }
      }
      // Check that we got an event back about the ping out.
      let msg = recv.next().await.unwrap();
      if let ButtplugServerMessage::Error(e) = msg {
        if messages::ErrorCode::ErrorPing != e.error_code {
          panic!("Didn't get a ping error");
        }
      } else {
        panic!("Didn't get an error message back");
      }
    });
  }

  #[test]
  fn test_device_stop_on_ping_timeout() {
    async_manager::block_on(async {
      let (mut server, mut recv) = ButtplugServer::new("Test Server", 100);
      let helper = server.add_test_comm_manager();
      // TODO This should probably use a test protocol we control, not the aneros protocol
      let device = helper.add_ble_device("Massage Demo").await;

      let msg =
        messages::RequestServerInfo::new("Test Client", BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION);
      let mut reply = server.parse_message(msg.into()).await;
      assert!(reply.is_ok(), format!("Should get back ok: {:?}", reply));
      reply = server
        .parse_message(messages::StartScanning::default().into())
        .await;
      assert!(reply.is_ok(), format!("Should get back ok: {:?}", reply));
      // Check that we got an event back about a new device.
      let msg = recv.next().await.unwrap();
      let device_index;
      if let ButtplugServerMessage::DeviceAdded(da) = msg {
        assert_eq!(da.device_name, "Aneros Vivi");
        device_index = da.device_index;
        println!("{:?}", da);
      } else {
        panic!(format!(
          "Returned message was not a DeviceAdded message or timed out: {:?}",
          msg
        ));
      }
      
      server
        .parse_message(
          messages::VibrateCmd::new(device_index, vec![messages::VibrateSubcommand::new(0, 0.5)]).into(),
        )
        .await
        .unwrap();
      let command_receiver = device.get_endpoint_channel(&Endpoint::Tx).unwrap().receiver;
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF1, 63], false)),
      )
      .await;
      // Wait out the ping, we should get a stop message.
      let mut i = 0u32;
      while command_receiver.is_empty() {
        Delay::new(Duration::from_millis(150)).await;
        // Breaks out of loop if we wait for too long.
        i += 1;
        assert!(i < 10, "Slept for too long while waiting for stop command!");
      }
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF1, 0], false)),
      )
      .await;
    });
  }
