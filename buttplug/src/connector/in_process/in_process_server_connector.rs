use super::super::ButtplugServerConnector;
use crate::{
  core::messages::{
  ButtplugClientInMessage,
  ButtplugClientOutMessage,
  ButtplugInMessage,
  ButtplugMessage,
  ButtplugOutMessage,
},
server::ButtplugServer
};
use async_std::{
  prelude::StreamExt,
  sync::{channel, Receiver},
  task,
};
use async_trait::async_trait;
use std::convert::TryInto;

pub struct ButtplugInProcessServerConnector {
  server: ButtplugServer,
}

impl ButtplugInProcessServerConnector {
  pub fn new(name: &str, max_ping_time: u128) -> (Self, Receiver<ButtplugClientOutMessage>) {
    let (send, recv) = channel(256);
    let (server, mut recv_server) = ButtplugServer::new(name, max_ping_time);

    task::spawn(async move {
      while let Some(event) = recv_server.next().await {
        let converted_event = ButtplugInProcessServerConnector::convert_outgoing(event);
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
impl ButtplugServerConnector for ButtplugInProcessServerConnector {
  type Input = ButtplugClientInMessage;
  type Output = ButtplugClientOutMessage;

  async fn parse_message(&mut self, msg: Self::Input) -> Self::Output {
    let input = ButtplugInProcessServerConnector::convert_incoming(msg.clone());
    let mut output = self.server.parse_message(&input).await.unwrap();
    output.set_id(input.get_id());
    ButtplugInProcessServerConnector::convert_outgoing(output)
  }

  fn server_ref(&mut self) -> &mut ButtplugServer {
    &mut self.server
  }
}
