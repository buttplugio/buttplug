use async_trait::async_trait;
use super::ButtplugServer;
use crate::core::{
    messages::{self, ButtplugClientOutMessage, ButtplugClientInMessage, ButtplugInMessage, ButtplugOutMessage, 
        ButtplugSpecV2InMessage, ButtplugSpecV2OutMessage, ButtplugSpecV1InMessage, ButtplugSpecV1OutMessage,
        ButtplugSpecV0InMessage, ButtplugSpecV0OutMessage },
    errors::{ButtplugError, ButtplugMessageError},
};
use async_std::{
    prelude::StreamExt,
    sync::{channel, Receiver},
    task,
};
use serde::{Deserialize};
use std::convert::TryInto;

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

    fn deserialize<T>(msg: String) -> Result<T, ButtplugError>
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
        if self.message_version.is_none() {
            let msg_union = ButtplugJSONServerWrapper::deserialize::<ButtplugSpecV2InMessage>(msg)?;
            if let ButtplugSpecV2InMessage::RequestServerInfo(rsi) = msg_union {
                self.message_version = Some(rsi.message_version);
            }
        }

        Ok(ButtplugInMessage::Ping(messages::Ping::default()))
    }

    fn convert_outgoing(&self, msg: ButtplugOutMessage) -> String {
        //msg.try_into().unwrap()
        "test".to_string()
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

    async fn parse_message(&mut self, msg: Self::Input) -> Self::Output {
        let input = self.convert_incoming(msg);
        let output = self.server.parse_message(&input.unwrap()).await.unwrap();
        self.convert_outgoing(output)
    }

    fn server_ref(&'a mut self) -> &'a mut ButtplugServer {
        &mut self.server
    }
}
