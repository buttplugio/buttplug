// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Handling of websockets using async-tungstenite

use super::transport::{ButtplugConnectorTransport, ButtplugTransportIncomingMessage, ButtplugTransportOutgoingMessage};
use crate::{
  connector::{ButtplugConnector, ButtplugConnectorError, ButtplugConnectorResultFuture},
  core::{
    errors::{ButtplugMessageError, ButtplugServerError},
    messages::{
      serializer::{
        ButtplugClientJSONSerializer,
        ButtplugMessageSerializer,
        ButtplugSerializedMessage,
      },
      ButtplugClientMessage,
      ButtplugCurrentSpecClientMessage,
      ButtplugCurrentSpecServerMessage,
      ButtplugMessage,
      ButtplugServerMessage,
    },
  },
  util::async_manager,
};
use async_channel::{bounded, Receiver, Sender};
use futures::{future::BoxFuture, FutureExt, StreamExt};
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
  Incoming(ButtplugTransportIncomingMessage),
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
  connector_incoming_sender: Sender<Result<InboundMessageType, ButtplugServerError>>,
  transport: TransportType,
  // Sends sorter processed messages to the transport.
  transport_outgoing_sender: Sender<ButtplugTransportOutgoingMessage>,
  // Takes data coming in from the transport.
  mut transport_incoming_recv: Receiver<ButtplugTransportIncomingMessage>,
) where
  TransportType: ButtplugConnectorTransport + 'static,
  SerializerType: ButtplugMessageSerializer<Inbound = InboundMessageType, Outbound = OutboundMessageType>
    + 'static,
  OutboundMessageType: ButtplugMessage + 'static,
  InboundMessageType: ButtplugMessage + 'static, //From<Error> + 'static,
{
  // Message sorter that receives messages that come in from the client.
  let mut serializer = SerializerType::default();
  loop {
    // We use two Options instead of an enum because we may never get anything.
    //
    // For the type, we will get back one of two things: Either a serialized
    // incoming message from the transport for the connector, or an outgoing
    // message from the connector to go to the transport.
    let mut stream_return = select! {
      // Catch messages coming in from the transport.
      transport = transport_incoming_recv.next().fuse() =>
      match transport {
        Some(msg) => StreamValue::Incoming(msg),
        None => StreamValue::NoValue,
      },
      connector = connector_outgoing_recv.next().fuse() =>
      match connector {
        // Catch messages that need to be sent out through the connector.
        Some(msg) => StreamValue::Outgoing(msg),
        None => StreamValue::NoValue,
      }
    };
    match stream_return {
      // If we get NoValue back, it means one side closed, so the other should
      // too.
      StreamValue::NoValue => break,
      // If we get incoming back, it means we've received something from the
      // server. See if we have a matching future, else send whatever we got as
      // an event.
      StreamValue::Incoming(remote_msg) => {
        match remote_msg {
          ButtplugTransportIncomingMessage::Message(serialized_msg) => {
            match serializer.deserialize(serialized_msg) {
              Ok(array) => {
                for smsg in array {
                  // TODO THIS SHOULD CONVERT ERROR MESSAGES FIRST.
                  if connector_incoming_sender.send(Ok(smsg)).await.is_err() {
                    error!("Connector has disconnected, ending remote connector loop.");
                    return;
                  }
                }
              }
              Err(e) => {
                let error_str =
                  format!("Got invalid messages from remote Buttplug Server: {:?}", e);
                error!("{}", error_str);
                let _ = connector_incoming_sender
                  .send(Err(
                    ButtplugMessageError::MessageSerializationError(e).into(),
                  ))
                  .await;
              }
            }
          }
          ButtplugTransportIncomingMessage::Close(s) => {
            info!("Connector closing connection {}", s);
            break;
          }
          // TODO We should probably make connecting an event?
          ButtplugTransportIncomingMessage::Connected => {}
          // TODO We should probably figure out what this even does?
          ButtplugTransportIncomingMessage::Error(_) => {}
        }
      }
      // If we receive something from the client, register it with our sorter
      // then let the connector figure out what to do with it.
      StreamValue::Outgoing(ref mut buttplug_msg) => {
        match buttplug_msg {
          ButtplugRemoteConnectorMessage::Message(msg) => {
            // Create future sets our message ID, so make sure this
            // happens before we send out the message.
            let serialized_msg = ButtplugTransportOutgoingMessage::Message(serializer.serialize(vec![msg.clone()]));
            if transport_outgoing_sender
              .send(serialized_msg)
              .await
              .is_err()
            {
              error!("Transport has disconnected, exiting remote connector loop.");
              return;
            }
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

pub type ButtplugRemoteClientConnector<
  TransportType,
  SerializerType = ButtplugClientJSONSerializer,
> = ButtplugRemoteConnector<
  TransportType,
  SerializerType,
  ButtplugCurrentSpecClientMessage,
  ButtplugCurrentSpecServerMessage,
>;

pub type ButtplugRemoteServerConnector<TransportType, SerializerType> = ButtplugRemoteConnector<
  TransportType,
  SerializerType,
  ButtplugServerMessage,
  ButtplugClientMessage,
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
  InboundMessageType: ButtplugMessage + 'static, //+ From<Error> + 'static,
{
  fn connect(
    &mut self,
  ) -> BoxFuture<
    'static,
    Result<Receiver<Result<InboundMessageType, ButtplugServerError>>, ButtplugConnectorError>,
  > {
    if self.transport.is_some() {
      // We can unwrap this because we just proved we had it.
      let transport = self.transport.take().unwrap();
      let (connector_outgoing_sender, connector_outgoing_receiver) = bounded(256);
      self.event_loop_sender = Some(connector_outgoing_sender);
      Box::pin(async move {
        match transport.connect().await {
          // If we connect successfully, we get back the channel from the transport
          // to send outgoing messages and receieve incoming events, all serialized.
          Ok((transport_outgoing_sender, transport_incoming_receiver)) => {
            let (connector_incoming_sender, connector_incoming_receiver) = bounded(256);
            async_manager::spawn(async move {
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
              .await
            })
            .unwrap();
            Ok(connector_incoming_receiver)
          }
          Err(e) => Err(e),
        }
      })
    } else {
      ButtplugConnectorError::ConnectorAlreadyConnected.into()
    }
  }

  fn disconnect(&self) -> ButtplugConnectorResultFuture {
    if let Some(ref sender) = self.event_loop_sender {
      let sender_clone = sender.clone();
      Box::pin(async move {
        sender_clone
          .send(ButtplugRemoteConnectorMessage::Close)
          .await
          .map_err(|_| ButtplugConnectorError::ConnectorNotConnected)
      })
    } else {
      ButtplugConnectorError::ConnectorNotConnected.into()
    }
  }

  fn send(&self, msg: OutboundMessageType) -> ButtplugConnectorResultFuture {
    if let Some(ref sender) = self.event_loop_sender {
      let sender_clone = sender.clone();
      Box::pin(async move {
        sender_clone
          .send(ButtplugRemoteConnectorMessage::Message(msg))
          .await
          .map_err(|_| ButtplugConnectorError::ConnectorNotConnected)
      })
    } else {
      ButtplugConnectorError::ConnectorNotConnected.into()
    }
  }
}
