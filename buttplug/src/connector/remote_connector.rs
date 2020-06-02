// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Handling of websockets using async-tungstenite

use super::ButtplugConnectorTransport;
use crate::{
  connector::{
    ButtplugConnector, ButtplugConnectorError, ButtplugConnectorResult, ButtplugTransportMessage,
  },
  core::{
    messages::{
      serializer::{ButtplugMessageSerializer, ButtplugSerializedMessage},
      ButtplugCurrentSpecClientMessage, ButtplugCurrentSpecServerMessage, ButtplugMessage,
    },
  },
};
use async_std::{
  prelude::FutureExt,
  sync::{channel, Receiver, Sender},
  task,
};
use async_trait::async_trait;
use futures_util::StreamExt;
use std::marker::PhantomData;

enum ButtplugRemoteConnectorMessage<T>
where
  T: ButtplugMessage + 'static,
{
  Message(T),
  Close,
}

enum StreamValue<T>
where
  T: ButtplugMessage + 'static,
{
  NoValue,
  Incoming(ButtplugTransportMessage),
  Outgoing(ButtplugRemoteConnectorMessage<T>),
}

async fn remote_connector_event_loop<
  TransportType,
  SerializerType,
  OutboundMessageType,
  InboundMessageType,
>(
  // Takes messages from the client
  mut connector_outgoing_recv: Receiver<ButtplugRemoteConnectorMessage<OutboundMessageType>>,
  // Sends messages not matched in the sorter to the client.
  connector_incoming_sender: Sender<InboundMessageType>,
  transport: TransportType,
  // Sends sorter processed messages to the transport.
  transport_outgoing_sender: Sender<ButtplugSerializedMessage>,
  // Takes data coming in from the transport.
  mut transport_incoming_recv: Receiver<ButtplugTransportMessage>,
) where
  TransportType: ButtplugConnectorTransport + 'static,
  SerializerType: ButtplugMessageSerializer<Inbound = InboundMessageType, Outbound = OutboundMessageType>
    + 'static,
  OutboundMessageType: ButtplugMessage + 'static,
  InboundMessageType: ButtplugMessage + 'static,
{
  // Message sorter that receives messages that come in from the client.
  let mut serializer = SerializerType::default();
  loop {
    // We use two Options instead of an enum because we may never get anything.
    //
    // For the type, we will get back one of two things: Either a serialized
    // incoming message from the transport for the connector, or an outgoing
    // message from the connector to go to the transport.
    let mut stream_return = async {
      // Catch messages coming in from the transport.
      match transport_incoming_recv.next().await {
        Some(msg) => StreamValue::Incoming(msg),
        None => StreamValue::NoValue,
      }
    }
    .race(async {
      match connector_outgoing_recv.next().await {
        // Catch messages that need to be sent out through the connector.
        Some(msg) => StreamValue::Outgoing(msg),
        None => StreamValue::NoValue,
      }
    })
    .await;
    match stream_return {
      // If we get NoValue back, it means one side closed, so the other should
      // too.
      StreamValue::NoValue => break,
      // If we get incoming back, it means we've received something from the
      // server. See if we have a matching future, else send whatever we got as
      // an event.
      StreamValue::Incoming(remote_msg) => {
        match remote_msg {
          ButtplugTransportMessage::Message(serialized_msg) => {
            match serializer.deserialize(serialized_msg) {
              Ok(array) => {
                for smsg in array {
                  connector_incoming_sender.send(smsg).await;
                }
              }
              Err(e) => {
                let error_str =
                  format!("Got invalid messages from remote Buttplug Server: {:?}", e);
                error!("{}", error_str);
                // TODO Implement error type to send back to connector
                /*
                let err_msg = messages::Error::from(
                  ButtplugError::ButtplugMessageError(ButtplugMessageError::new(&error_str)).into(),
                );
                connector_incoming_sender.send(err_msg.into()).await;
                */
              }
            }
          }
          ButtplugTransportMessage::Close(s) => {
            info!("Connector closing connection {}", s);
            break;
          }
          // TODO We should probably make connecting an event?
          ButtplugTransportMessage::Connected => {}
          // TODO We should probably figure out what this even does?
          ButtplugTransportMessage::Error(_) => {}
        }
      }
      // If we receive something from the client, register it with our sorter
      // then let the connector figure out what to do with it.
      StreamValue::Outgoing(ref mut buttplug_msg) => {
        match buttplug_msg {
          ButtplugRemoteConnectorMessage::Message(msg) => {
            // Create future sets our message ID, so make sure this
            // happens before we send out the message.
            let serialized_msg = serializer.serialize(vec![msg.clone()]);
            transport_outgoing_sender.send(serialized_msg).await;
          }
          ButtplugRemoteConnectorMessage::Close => {
            if let Err(e) = transport.disconnect().await {
              error!("Error disconnecting transport: {:?}", e);
            }
            break;
          }
        }
      }
    }
  }
}

pub type ButtplugRemoteClientConnector<TransportType, SerializerType> = ButtplugRemoteConnector<
  TransportType,
  SerializerType,
  ButtplugCurrentSpecClientMessage,
  ButtplugCurrentSpecServerMessage,
>;

pub struct ButtplugRemoteConnector<
  TransportType,
  SerializerType,
  OutboundMessageType,
  InboundMessageType,
> where
  TransportType: ButtplugConnectorTransport + 'static,
  SerializerType: ButtplugMessageSerializer<Inbound = InboundMessageType, Outbound = OutboundMessageType>
    + 'static,
  OutboundMessageType: ButtplugMessage + 'static,
  InboundMessageType: ButtplugMessage + 'static,
{
  /// Transport that the connector will use to communicate with the other
  /// connector.
  ///
  /// This is an option so that, if we connect successfully, we can `.take()`
  /// the value out of the option and send it to our event loop. This means if
  /// anyone tries to call connect twice, we'll fail (because we'll have no
  /// transport to connect to). It also limits the lifetime of the connector to
  /// the lifetime of the event loop, meaning if for any reason we exit, we make
  /// sure the transport is dropped.
  transport: Option<TransportType>,
  /// Sender for forwarding outgoing messages to the connector event loop.
  event_loop_sender: Option<Sender<ButtplugRemoteConnectorMessage<OutboundMessageType>>>,
  dummy_serializer: PhantomData<SerializerType>,
}

impl<TransportType, SerializerType, OutboundMessageType, InboundMessageType>
  ButtplugRemoteConnector<TransportType, SerializerType, OutboundMessageType, InboundMessageType>
where
  TransportType: ButtplugConnectorTransport + 'static,
  SerializerType: ButtplugMessageSerializer<Inbound = InboundMessageType, Outbound = OutboundMessageType>
    + 'static,
  OutboundMessageType: ButtplugMessage + 'static,
  InboundMessageType: ButtplugMessage + 'static,
{
  pub fn new(transport: TransportType) -> Self {
    Self {
      transport: Some(transport),
      event_loop_sender: None,
      dummy_serializer: PhantomData::default(),
    }
  }
}

#[async_trait]
impl<TransportType, SerializerType, OutboundMessageType, InboundMessageType>
  ButtplugConnector<OutboundMessageType, InboundMessageType>
  for ButtplugRemoteConnector<
    TransportType,
    SerializerType,
    OutboundMessageType,
    InboundMessageType,
  >
where
  TransportType: ButtplugConnectorTransport + 'static,
  SerializerType: ButtplugMessageSerializer<Inbound = InboundMessageType, Outbound = OutboundMessageType>
    + 'static,
  OutboundMessageType: ButtplugMessage + 'static,
  InboundMessageType: ButtplugMessage + 'static,
{
  async fn connect(&mut self) -> Result<Receiver<InboundMessageType>, ButtplugConnectorError> {
    if self.transport.is_some() {
      // We can unwrap this because we just proved we had it.
      let mut transport = self.transport.take().unwrap();
      match transport.connect().await {
        // If we connect successfully, we get back the channel from the transport
        // to send outgoing messages and receieve incoming events, all serialized.
        Ok((transport_outgoing_sender, transport_incoming_receiver)) => {
          // So we
          let (connector_outgoing_sender, connector_outgoing_receiver) = channel(256);
          let (connector_incoming_sender, connector_incoming_receiver) = channel(256);
          task::spawn(async move {
            remote_connector_event_loop::<
              TransportType,
              SerializerType,
              OutboundMessageType,
              InboundMessageType,
            >(
              connector_outgoing_receiver,
              connector_incoming_sender,
              transport,
              transport_outgoing_sender,
              transport_incoming_receiver,
            )
          });
          self.event_loop_sender = Some(connector_outgoing_sender);
          Ok(connector_incoming_receiver)
        }
        Err(e) => Err(e),
      }
    } else {
      Err(ButtplugConnectorError::new("Connector already connected."))
    }
  }

  async fn disconnect(&mut self) -> ButtplugConnectorResult {
    if let Some(ref mut sender) = self.event_loop_sender {
      sender.send(ButtplugRemoteConnectorMessage::Close).await;
      Ok(())
    } else {
      Err(ButtplugConnectorError::new("Connector not connected."))
    }
  }

  async fn send(&mut self, msg: OutboundMessageType) -> ButtplugConnectorResult {
    if let Some(ref mut sender) = self.event_loop_sender {
      sender
        .send(ButtplugRemoteConnectorMessage::Message(msg))
        .await;
      Ok(())
    } else {
      Err(ButtplugConnectorError::new("Connector not connected."))
    }
  }
}
