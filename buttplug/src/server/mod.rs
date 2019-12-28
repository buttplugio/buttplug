// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Handles client sessions, as well as discovery and communication with hardware.

pub mod device_manager;
pub mod device;
pub mod comm_managers;

use crate::core::{
    errors::*,
    messages::{self, ButtplugMessage, ButtplugMessageUnion, DeviceMessageInfo},
};
use async_std::sync::Sender;

pub enum ButtplugServerEvent {
    DeviceAdded(DeviceMessageInfo),
    DeviceRemoved(DeviceMessageInfo),
    DeviceMessage(ButtplugMessageUnion),
    ScanningFinished(),
    ServerError(ButtplugError),
    PingTimeout(),
    Log(messages::Log),
}

/// Represents a ButtplugServer.
pub struct ButtplugServer {
    server_name: String,
    server_spec_version: u32,
    client_spec_version: Option<u32>,
    client_name: Option<String>,
    max_ping_time: u32,
    _event_sender: Sender<ButtplugMessageUnion>,
}

impl ButtplugServer {
    pub fn new(
        name: &str,
        max_ping_time: u32,
        _event_sender: Sender<ButtplugMessageUnion>,
    ) -> Self {
        Self {
            server_name: name.to_string(),
            server_spec_version: 1,
            client_name: None,
            client_spec_version: None,
            max_ping_time,
            _event_sender,
        }
    }

    pub async fn parse_message(
        &mut self,
        msg: &ButtplugMessageUnion,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        match msg {
            ButtplugMessageUnion::RequestServerInfo(ref _s) => self.perform_handshake(_s),
            ButtplugMessageUnion::StartScanning(_) => {
                self.start_scanning().await?;
                Result::Ok(ButtplugMessageUnion::Ok(messages::Ok::new(msg.get_id())))
            }
            ButtplugMessageUnion::StopScanning(_) => {
                self.stop_scanning().await?;
                Result::Ok(ButtplugMessageUnion::Ok(messages::Ok::new(msg.get_id())))
            }
            ButtplugMessageUnion::RequestDeviceList(_) => {
                let mut list = messages::DeviceList::default();
                list.set_id(msg.get_id());
                Result::Ok(list.into())
            }
            _ => Result::Ok(ButtplugMessageUnion::Ok(messages::Ok::new(msg.get_id()))),
        }
    }

    fn perform_handshake(
        &mut self,
        msg: &messages::RequestServerInfo,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        if self.server_spec_version < msg.message_version {
            return Result::Err(ButtplugError::ButtplugHandshakeError(
                ButtplugHandshakeError {
                    message: format!(
                        "Server version ({}) must be equal to or greater than client version ({}).",
                        self.server_spec_version, msg.message_version
                    ),
                },
            ));
        }
        self.client_name = Option::Some(msg.client_name.clone());
        self.client_spec_version = Option::Some(msg.message_version);
        Result::Ok(
            messages::ServerInfo::new(
                &self.server_name,
                self.server_spec_version,
                self.max_ping_time,
            )
            .into(),
        )
    }

    async fn start_scanning(&self) -> Result<(), ButtplugError> {
        Ok(())
    }

    async fn stop_scanning(&self) -> Result<(), ButtplugError> {
        Ok(())
    }

    // async fn wait_for_event(&self) -> Result<ButtplugServerEvent> {
    // }
}

#[cfg(test)]
mod test {
    use super::*;
    use async_std::{sync::channel, task};

    async fn test_server_setup(msg_union: &messages::ButtplugMessageUnion) -> ButtplugServer {
        let (send, _) = channel(256);
        let mut server = ButtplugServer::new("Test Server", 0, send);
        assert_eq!(server.server_name, "Test Server");
        match server.parse_message(&msg_union).await.unwrap() {
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
        let (send, _) = channel(256);
        let mut server = ButtplugServer::new("Test Server", 0, send);
        let msg = messages::RequestServerInfo::new("Test Client", server.server_spec_version + 1);
        let msg_union = ButtplugMessageUnion::RequestServerInfo(msg);
        task::block_on(async {
            assert!(
                server.parse_message(&msg_union).await.is_err(),
                "Client having higher version than server should fail"
            );
        });
    }
}
