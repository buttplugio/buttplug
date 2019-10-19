use crate::core::errors::*;
use crate::core::messages;
use crate::core::messages::ButtplugMessage;
use crate::core::messages::ButtplugMessageUnion;
use futures_channel::mpsc;

pub struct ButtplugServer {
    server_name: String,
    server_spec_version: u32,
    client_spec_version: Option<u32>,
    client_name: Option<String>,
    max_ping_time: u32,
    event_sender: mpsc::UnboundedSender<ButtplugMessageUnion>,
}

impl ButtplugServer {
    pub fn new(
        name: &str,
        max_ping_time: u32,
        event_sender: mpsc::UnboundedSender<ButtplugMessageUnion>,
    ) -> ButtplugServer {
        ButtplugServer {
            server_name: name.to_string(),
            server_spec_version: 1,
            client_name: None,
            client_spec_version: None,
            max_ping_time: max_ping_time,
            event_sender,
        }
    }

    pub async fn send_message(
        &mut self,
        msg: &ButtplugMessageUnion,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        match msg {
            ButtplugMessageUnion::RequestServerInfo(ref _s) => self.perform_handshake(_s),
            ButtplugMessageUnion::StartScanning(_) => self.start_scanning().await.map_or_else(
                || Result::Ok(ButtplugMessageUnion::Ok(messages::Ok::new(msg.get_id()))),
                |x| Result::Err(x),
            ),
            ButtplugMessageUnion::StopScanning(_) => self.stop_scanning().await.map_or_else(
                || Result::Ok(ButtplugMessageUnion::Ok(messages::Ok::new(msg.get_id()))),
                |x| Result::Err(x),
            ),
            _ => return Result::Ok(ButtplugMessageUnion::Ok(messages::Ok::new(msg.get_id()))),
        }
    }

    fn perform_handshake(
        &mut self,
        msg: &messages::RequestServerInfo,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        if self.server_spec_version < msg.message_version {
            return Result::Err(ButtplugError::ButtplugInitError(ButtplugInitError {
                message: format!(
                    "Server version ({}) must be equal to or greater than client version ({}).",
                    self.server_spec_version, msg.message_version
                ),
            }));
        }
        self.client_name = Option::Some(msg.client_name.clone());
        self.client_spec_version = Option::Some(msg.message_version);
        Result::Ok(
            messages::ServerInfo::new(
                &self.server_name,
                self.server_spec_version,
                self.max_ping_time,
            )
            .as_union(),
        )
    }

    async fn start_scanning(&self) -> Option<ButtplugError> {
        None
    }

    async fn stop_scanning(&self) -> Option<ButtplugError> {
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use async_std::task;

    async fn test_server_setup(msg_union: &messages::ButtplugMessageUnion) -> ButtplugServer {
        let (send, recv) = mpsc::unbounded();
        let mut server = ButtplugServer::new("Test Server", 0, send);
        assert_eq!(server.server_name, "Test Server");
        match server.send_message(&msg_union).await.unwrap() {
            ButtplugMessageUnion::ServerInfo(_s) => {
                assert_eq!(_s, messages::ServerInfo::new("Test Server", 1, 0))
            }
            _ => assert!(false, "Should've received ok"),
        }
        server
    }

    #[test]
    fn test_server_handshake() {
        let msg = messages::RequestServerInfo::new("Test Client", 1);
        let msg_union = ButtplugMessageUnion::RequestServerInfo(msg);
        task::block_on(async {
            let server = test_server_setup(&msg_union).await;
            assert_eq!(server.client_name.unwrap(), "Test Client");
        });
    }

    #[test]
    fn test_server_version_lt() {
        let msg = messages::RequestServerInfo::new("Test Client", 0);
        let msg_union = ButtplugMessageUnion::RequestServerInfo(msg);
        task::block_on(async {
            test_server_setup(&msg_union).await;
        });
    }

    #[test]
    fn test_server_version_gt() {
        let (send, recv) = mpsc::unbounded();
        let mut server = ButtplugServer::new("Test Server", 0, send);
        let msg = messages::RequestServerInfo::new("Test Client", server.server_spec_version + 1);
        let msg_union = ButtplugMessageUnion::RequestServerInfo(msg);
        task::block_on(async {
            assert!(
                server.send_message(&msg_union).await.is_err(),
                "Client having higher version than server should fail"
            );
        });
    }
}
