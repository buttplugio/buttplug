mod json_wrapper;

pub use json_wrapper::ButtplugJSONServerWrapper;
use async_trait::async_trait;
use super::ButtplugServer;
use crate::core::{
    messages::{ButtplugClientOutMessage, ButtplugClientInMessage, ButtplugInMessage, ButtplugOutMessage},
};
use async_std::{
    prelude::StreamExt,
    sync::{channel, Receiver},
    task,
};
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
