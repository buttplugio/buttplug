use async_trait::async_trait;
use super::ButtplugServerWrapper;
use crate::{
    server::ButtplugServer,
    core::{
    messages::{self, ButtplugInMessage, ButtplugOutMessage, 
        ButtplugSpecV2InMessage, ButtplugSpecV2OutMessage, ButtplugSpecV1InMessage, ButtplugSpecV1OutMessage,
        ButtplugSpecV0InMessage, ButtplugSpecV0OutMessage, ButtplugMessage, ButtplugMessageSpecVersion },
    errors::{ButtplugError, ButtplugMessageError, ButtplugHandshakeError},
    }
};
use async_std::{
    prelude::StreamExt,
    sync::{channel, Receiver, Sender},
    task,
};
use std::convert::{TryFrom};

pub struct ButtplugJSONServerWrapper {
    server: ButtplugServer,
    message_version: Option<messages::ButtplugMessageSpecVersion>,
    recv_server: Option<Receiver<ButtplugOutMessage>>,
    event_sender: Option<Sender<String>>,
}

impl ButtplugJSONServerWrapper {
    // This won't be called anywhere inside of the library, but we'll need it anyways.
    #[allow(dead_code)]
    pub fn new(name: &str,
        max_ping_time: u128
    ) -> (Self, Receiver<String>) {
        let (send, recv) = channel(256);
        let (server, recv_server) = ButtplugServer::new(name, max_ping_time);

        (Self { server, message_version: None, recv_server: Some(recv_server), event_sender: Some(send)}, recv)
    }

    pub(crate) fn deserialize<T>(msg: String) -> Result<T, ButtplugError>
        where T: serde::de::DeserializeOwned + Clone {
        serde_json::from_str::<Vec<T>>(&msg)
            .and_then(|msg_vec| Ok(msg_vec[0].clone()))
            .map_err(|e| ButtplugMessageError::new(&e.to_string()).into())
    }

    fn convert_incoming(&mut self, msg: String) -> Result<ButtplugInMessage, ButtplugError> {
        // If we don't have a message version yet, we need to parse this as a
        // RequestServerInfo message to get the version. RequestServerInfo can
        // always be parsed as the latest message version, as we keep it
        // compatible across versions via serde options.
        if let Some(version) = self.message_version {
            match version {
                ButtplugMessageSpecVersion::Version0 => {
                    let bp_msg = ButtplugJSONServerWrapper::deserialize::<ButtplugSpecV0InMessage>(msg)?;
                    Ok(bp_msg.into())
                },
                ButtplugMessageSpecVersion::Version1 => {
                    let bp_msg = ButtplugJSONServerWrapper::deserialize::<ButtplugSpecV1InMessage>(msg)?;
                    Ok(bp_msg.into())
                }
                ButtplugMessageSpecVersion::Version2 => {
                    let bp_msg = ButtplugJSONServerWrapper::deserialize::<ButtplugSpecV2InMessage>(msg)?;
                    Ok(bp_msg.into())
                }
            }
        } else {
            let msg_union = ButtplugJSONServerWrapper::deserialize::<ButtplugSpecV2InMessage>(msg)?;
            let version;
            if let ButtplugSpecV2InMessage::RequestServerInfo(rsi) = &msg_union {
                info!("Setting JSON Wrapper message version to {}", rsi.message_version); 
                self.message_version = Some(rsi.message_version);
                version = rsi.message_version;
            } else {
                return Err(ButtplugError::ButtplugHandshakeError(ButtplugHandshakeError::new("First message received must be a RequestServerInfo message.")));
            }
            let mut recv_server = self.recv_server.take().unwrap();
            let send = self.event_sender.take().unwrap();
            task::spawn(async move {
                while let Some(event) = recv_server.next().await {
                    let converted_event = ButtplugJSONServerWrapper::convert_outgoing_associated(version, event);
                    send.send(converted_event).await;
                }
            });
            Ok(msg_union.into())
        }
    }

    fn convert_outgoing_associated(version: ButtplugMessageSpecVersion, msg: ButtplugOutMessage) -> String {
            match version {
                ButtplugMessageSpecVersion::Version0 => {
                    match ButtplugSpecV0OutMessage::try_from(msg) {
                        Ok(msgv0) => msgv0.as_protocol_json(),
                        Err(err) => ButtplugSpecV0OutMessage::Error(ButtplugError::ButtplugMessageError(err).into()).as_protocol_json()
                    }
                },
                ButtplugMessageSpecVersion::Version1 => {
                    match ButtplugSpecV1OutMessage::try_from(msg) {
                        Ok(msgv1) => msgv1.as_protocol_json(),
                        Err(err) => ButtplugSpecV1OutMessage::Error(ButtplugError::ButtplugMessageError(err).into()).as_protocol_json()
                    }
                }
                ButtplugMessageSpecVersion::Version2 => {
                    match ButtplugSpecV2OutMessage::try_from(msg) {
                        Ok(msgv2) => msgv2.as_protocol_json(),
                        Err(err) => ButtplugSpecV2OutMessage::Error(ButtplugError::ButtplugMessageError(err).into()).as_protocol_json()
                    }
                }
            }
    }

    fn convert_outgoing(&self, msg: ButtplugOutMessage) -> String {
        if let Some(version) = self.message_version {
            ButtplugJSONServerWrapper::convert_outgoing_associated(version, msg)
        } else {
            // In the rare event that there is a problem with the
            // RequestServerInfo message (so we can't set up our known spec
            // version), just encode to the latest and return.
            if let ButtplugOutMessage::Error(_) = &msg {
                ButtplugJSONServerWrapper::convert_outgoing_associated(ButtplugMessageSpecVersion::Version2, msg.clone())
            } else {
                // If we don't even have enough info to know which message
                // version to convert to, consider this a handshake error.
                ButtplugOutMessage::Error(ButtplugError::ButtplugHandshakeError(ButtplugHandshakeError::new("Got outgoing message before version was set.")).into()).as_protocol_json()
            }
        }
    }
}

#[async_trait]
impl<'a> ButtplugServerWrapper<'a> for ButtplugJSONServerWrapper {
    // This was the only way I could figure out how to get a string in here
    // without ended up in lifetime hell. Trying to take a reference here is
    // really difficult because we do our message preparation in an async
    // function. It could be worth dividing that out into its own function,
    // but would mean more struct exposure.
    type Input = String;
    type Output = String;

    async fn parse_message(&mut self, str_msg: Self::Input) -> Self::Output {
        match self.convert_incoming(str_msg) {
            Ok(msg) => {
                let server_response = self.server.parse_message(&msg).await.unwrap();
                self.convert_outgoing(server_response) 
            },
            Err(err) => self.convert_outgoing(ButtplugOutMessage::Error(err.into()))
        }
    }

    fn server_ref(&'a mut self) -> &'a mut ButtplugServer {
        &mut self.server
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use async_std::task;

    #[test]
    fn test_correct_message_version() {
        let (mut json_wrapper, _) = ButtplugJSONServerWrapper::new("Test Wrapper", 0);
        let json = r#"[{
            "RequestServerInfo": {
                "Id": 1,
                "ClientName": "Test Client",
                "MessageVersion": 2
            }
        }]"#;
        task::block_on(async move {
            let msg = json_wrapper.parse_message(json.to_owned()).await;
            let err_msg = ButtplugJSONServerWrapper::deserialize::<ButtplugSpecV2OutMessage>(msg).unwrap();
            if let ButtplugSpecV2OutMessage::ServerInfo(e) = err_msg {
                assert!(true, format!("Correct message! {:?}", e));
            } else {
                assert!(false, format!("Wrong message! {:?}", err_msg));
            }
        });
    }

    #[test]
    fn test_wrong_message_version() {
        let (mut json_wrapper, _) = ButtplugJSONServerWrapper::new("Test Wrapper", 0);
        let json = r#"[{
            "RequestServerInfo": {
                "Id": 1,
                "ClientName": "Test Client",
                "MessageVersion": 100
            }
        }]"#;
        task::block_on(async move {
            let msg = json_wrapper.parse_message(json.to_owned()).await;
            let err_msg = ButtplugJSONServerWrapper::deserialize::<ButtplugSpecV2OutMessage>(msg).unwrap();
            if let ButtplugSpecV2OutMessage::Error(e) = err_msg {
                assert!(true, "Correct message! {:?}", e);
            } else {
                assert!(false, "Wrong message!");
            }
        });
    }
}