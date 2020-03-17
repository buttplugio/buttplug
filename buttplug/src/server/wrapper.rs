use async_trait::async_trait;
use super::ButtplugServer;
use crate::core::{
    messages::{self, ButtplugClientOutMessage, ButtplugClientInMessage, ButtplugInMessage, ButtplugOutMessage, 
        ButtplugSpecV2InMessage, ButtplugSpecV2OutMessage, ButtplugSpecV1InMessage, ButtplugSpecV1OutMessage,
        ButtplugSpecV0InMessage, ButtplugSpecV0OutMessage, ButtplugMessage, ButtplugMessageSpecVersion },
    errors::{ButtplugError, ButtplugMessageError},
};
use async_std::{
    prelude::StreamExt,
    sync::{channel, Receiver},
    task,
};
use std::convert::{TryFrom, TryInto};

#[async_trait]
pub trait ButtplugServerWrapper<'a> {
    type Input;
    type Output;

    async fn parse_message(&mut self, msg: Self::Input) -> Self::Output;
    fn server_ref(&'a mut self) -> &'a mut ButtplugServer;
}

pub struct ButtplugInProcessServerWrapper {
    server: ButtplugServer
}

impl ButtplugInProcessServerWrapper {
    pub fn new(name: &str,
        max_ping_time: u128
    ) -> (Self, Receiver<ButtplugClientOutMessage>) {
        let (send, recv) = channel(256);
        let (server, mut recv_server) = ButtplugServer::new(name, max_ping_time);
        
        task::spawn(async move {
            while let Some(event) = recv_server.next().await {
                let converted_event = ButtplugInProcessServerWrapper::convert_outgoing(event);
                send.send(converted_event).await;
            }
        });

        (Self { server }, recv)
    }

    fn convert_incoming(msg: ButtplugClientInMessage) -> ButtplugInMessage {
        msg.into()
    }

    fn convert_outgoing(msg: ButtplugOutMessage) -> ButtplugClientOutMessage {
        msg.try_into().unwrap()
    }
}

#[async_trait]
impl<'a> ButtplugServerWrapper<'a> for ButtplugInProcessServerWrapper {
    type Input = ButtplugClientInMessage; 
    type Output = ButtplugClientOutMessage;

    async fn parse_message(&mut self, msg: Self::Input) -> Self::Output {
        let input = ButtplugInProcessServerWrapper::convert_incoming(msg.clone());
        let output = self.server.parse_message(&input).await.unwrap();
        ButtplugInProcessServerWrapper::convert_outgoing(output)
    }

    fn server_ref(&'a mut self) -> &'a mut ButtplugServer {
        &mut self.server
    }
}

#[derive(Clone, Debug)]
pub struct JSONStringWrapper {
    pub json_str: String
}

pub struct ButtplugJSONServerWrapper {
    server: ButtplugServer,
    message_version: Option<messages::ButtplugMessageSpecVersion>
}

impl ButtplugJSONServerWrapper {
    // This won't be called anywhere inside of the library, but we'll need it anyways.
    #[allow(dead_code)]
    pub fn new(name: &str,
        max_ping_time: u128
    ) -> (Self, Receiver<ButtplugClientOutMessage>) {
        let (send, recv) = channel(256);
        let (server, mut recv_server) = ButtplugServer::new(name, max_ping_time);
        
        task::spawn(async move {
            while let Some(event) = recv_server.next().await {
                let converted_event = ButtplugInProcessServerWrapper::convert_outgoing(event);
                send.send(converted_event).await;
            }
        });

        (Self { server, message_version: None }, recv)
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
            if let ButtplugSpecV2InMessage::RequestServerInfo(rsi) = &msg_union {
                self.message_version = Some(rsi.message_version);
            }
            Ok(msg_union.into())
        }
    }

    fn convert_outgoing(&self, msg: ButtplugOutMessage) -> String {
        if let Some(version) = self.message_version {
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
        } else {
            ButtplugOutMessage::Error(ButtplugError::ButtplugMessageError(ButtplugMessageError::new("Got outgoing message before incoming?!")).into()).as_protocol_json()
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