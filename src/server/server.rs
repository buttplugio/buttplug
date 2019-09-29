use crate::core::messages;
use crate::core::errors::*;
use crate::core::messages::ButtplugMessageUnion;
use crate::core::messages::ButtplugMessage;

pub struct ButtplugServer {
    server_name: String,
    server_spec_version: u32,
    client_spec_version: Option<u32>,
    client_name: Option<String>,
    max_ping_time: u32,
}

impl ButtplugServer {
    pub fn new(name: &str, max_ping_time: u32) -> ButtplugServer {
        ButtplugServer {
            server_name: name.to_string(),
            server_spec_version: 1,
            client_name: None,
            client_spec_version: None,
            max_ping_time: max_ping_time,
        }
    }

    pub fn send_message(&mut self, msg: &ButtplugMessageUnion) -> Result<ButtplugMessageUnion, ButtplugError> {
        match msg {
            ButtplugMessageUnion::RequestServerInfo(ref _s) => self.perform_handshake(_s),
            ButtplugMessageUnion::StartScanning (_) => self.start_scanning(),
            ButtplugMessageUnion::StopScanning (_) => self.stop_scanning(),
            _ => return Result::Ok(ButtplugMessageUnion::Ok(messages::Ok::new())),
        }
    }

    fn perform_handshake(&mut self, msg: &messages::RequestServerInfo)
                         -> Result<ButtplugMessageUnion, ButtplugError> {
        if self.server_spec_version < msg.message_version {
            return Result::Err(
                ButtplugError::ButtplugInitError(
                    ButtplugInitError {
                        message: format!("Server version ({}) must be equal to or greater than client version ({}).",
                                         self.server_spec_version,
                                         msg.message_version)
                    }));
        }
        self.client_name = Option::Some(msg.client_name.clone());
        self.client_spec_version = Option::Some(msg.message_version);
        Result::Ok(messages::ServerInfo::new(&self.server_name, self.server_spec_version, self.max_ping_time).as_union())
    }

    fn start_scanning(&self) -> Result<ButtplugMessageUnion, ButtplugError> {
        Result::Ok(messages::Ok::new().as_union())
    }

    fn stop_scanning(&self) -> Result<ButtplugMessageUnion, ButtplugError> {
        Result::Ok(messages::Ok::new().as_union())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_server_setup(msg_union: &messages::ButtplugMessageUnion) -> ButtplugServer {
        let mut server = ButtplugServer::new("Test Server", 0);
        assert_eq!(server.server_name, "Test Server");
        match server.send_message(&msg_union).unwrap() {
            ButtplugMessageUnion::ServerInfo (_s) => assert_eq!(_s, messages::ServerInfo::new("Test Server", 1, 0)),
            _ =>  assert!(false, "Should've received ok"),
        }
        server
    }

    #[test]
    fn test_server_handshake() {
        let msg = messages::RequestServerInfo::new("Test Client", 1);
        let msg_union = ButtplugMessageUnion::RequestServerInfo(msg);
        let server = test_server_setup(&msg_union);
        assert_eq!(server.client_name.unwrap(), "Test Client");
    }

    #[test]
    fn test_server_version_lt() {
        let msg = messages::RequestServerInfo::new("Test Client", 0);
        let msg_union = ButtplugMessageUnion::RequestServerInfo(msg);
        test_server_setup(&msg_union);
    }

    #[test]
    fn test_server_version_gt() {
        let mut server = ButtplugServer::new("Test Server", 0);
        let msg = messages::RequestServerInfo::new("Test Client", server.server_spec_version + 1);
        let msg_union = ButtplugMessageUnion::RequestServerInfo(msg);
        assert!(server.send_message(&msg_union).is_err(), "Client having higher version than server should fail");
    }
}
