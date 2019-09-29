use crate::core::messages;
use crate::core::errors::*;
use crate::core::messages::ButtplugMessageUnion;

struct ButtplugServer {
    server_name: String,
    server_spec_version: u32,
    client_spec_version: Option<u32>,
    client_name: Option<String>,
}

impl ButtplugServer {
    pub fn new(name: String) -> ButtplugServer {
        ButtplugServer {
            server_name: name,
            server_spec_version: 1,
            client_name: None,
            client_spec_version: None,
        }
    }

    pub fn send_message(&mut self, msg: &ButtplugMessageUnion) -> Result<ButtplugMessageUnion, ButtplugError> {
        let err = match msg {
            ButtplugMessageUnion::RequestServerInfo(ref _s) => self.perform_handshake(_s),
            ButtplugMessageUnion::StartScanning (_) => self.start_scanning(),
            ButtplugMessageUnion::StopScanning (_) => self.stop_scanning(),
            _ => return Result::Ok(ButtplugMessageUnion::Ok(messages::Ok { id: 0 })),
        };

        match err {
            Some (_s) => return Result::Err(_s),
            None => return Result::Ok(ButtplugMessageUnion::Ok(messages::Ok { id: 0 }))
        }
    }

    fn perform_handshake(&mut self, msg: &messages::RequestServerInfo)
                         -> Option<ButtplugError> {
        if self.server_spec_version < msg.message_version {
            return Option::Some(
                ButtplugError::ButtplugInitError(
                    ButtplugInitError {
                        message: format!("Server version ({}) must be equal to or greater than client version ({}).",
                                         self.server_spec_version,
                                         msg.message_version)
                    }));
        }
        self.client_name = Option::Some(msg.client_name.clone());
        self.client_spec_version = Option::Some(msg.message_version);
        Option::None
    }

    fn start_scanning(&self) -> Option<ButtplugError> {
        Option::None
    }

    fn stop_scanning(&self) -> Option<ButtplugError> {
        Option::None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_server_setup(msg_union: &messages::ButtplugMessageUnion) -> ButtplugServer {
        let mut server = ButtplugServer::new("Test Server".to_string());
        assert_eq!(server.server_name, "Test Server");
        match server.send_message(&msg_union).unwrap() {
            ButtplugMessageUnion::Ok (_s) => assert_eq!(_s, messages::Ok { id: 0 }),
            _ =>  assert!(false, "Should've received ok"),
        }
        server
    }

    #[test]
    fn test_server_handshake() {
        let msg = messages::RequestServerInfo {
            id: 0,
            client_name: "Test Client".to_string(),
            message_version: 0,
        };
        let msg_union = ButtplugMessageUnion::RequestServerInfo(msg);
        let server = test_server_setup(&msg_union);
        assert_eq!(server.client_name.unwrap(), "Test Client");
    }

    #[test]
    fn test_server_version_lt() {
        let msg = messages::RequestServerInfo {
            id: 0,
            client_name: "Test Client".to_string(),
            message_version: 0,
        };
        let msg_union = ButtplugMessageUnion::RequestServerInfo(msg);
        test_server_setup(&msg_union);
    }

    #[test]
    fn test_server_version_gt() {
        let mut server = ButtplugServer::new("Test Server".to_string());
        let msg = messages::RequestServerInfo {
            id: 0,
            client_name: "Test Client".to_string(),
            message_version: server.server_spec_version + 1,
        };
        let msg_union = ButtplugMessageUnion::RequestServerInfo(msg);
        assert!(server.send_message(&msg_union).is_err(), "Client having higher version than server should fail");
    }
}
