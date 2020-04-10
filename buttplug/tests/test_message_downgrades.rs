extern crate buttplug;

#[cfg(test)]
mod test {
  use buttplug::{
    core::messages::BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION,
    server::ButtplugServerWrapper,
    server::wrapper::ButtplugJSONServerWrapper,
  };
  use async_std::task;

  #[test]
  fn test_version0_connection() {
    let _ = env_logger::builder().is_test(true).try_init();
    task::block_on(async {
      let (mut server, _) = ButtplugJSONServerWrapper::new("Test Server", 0);
      let rsi = r#"[{"RequestServerInfo":{"Id": 1, "ClientName": "Test Client"}}]"#;
      let output = server.parse_message(rsi.to_owned()).await;
      assert_eq!(output, format!(r#"[{{"ServerInfo":{{"Id":0,"MajorVersion":0,"MinorVersion":0,"BuildVersion":0,"MessageVersion":{},"MaxPingTime":0,"ServerName":"Test Server"}}}}]"#, BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION as u32));
    });
  }

  #[test]
  fn test_version2_connection() {
    let _ = env_logger::builder().is_test(true).try_init();
    task::block_on(async {
      let (mut server, _) = ButtplugJSONServerWrapper::new("Test Server", 0);
      let rsi = r#"[{"RequestServerInfo":{"Id": 1, "ClientName": "Test Client", "MessageVersion": 2}}]"#;
      let output = server.parse_message(rsi.to_owned()).await;
      assert_eq!(output, format!(r#"[{{"ServerInfo":{{"Id":0,"MessageVersion":{},"MaxPingTime":0,"ServerName":"Test Server"}}}}]"#, BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION as u32));
    });
  }
}