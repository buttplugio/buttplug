// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::super::{
  ButtplugClientConnectorError,  ClientConnectorMessageSorter,
};
use crate::{
  client::{ButtplugClientMessageFuture, ButtplugClientMessageFuturePair,
    ButtplugInternalClientMessageResult},
    core::{
  errors::{ButtplugError, ButtplugMessageError},
  messages::{ButtplugClientInMessage, ButtplugClientOutMessage, serializer::{ButtplugMessageSerializer, ButtplugSerializedMessage}},
    },
};
use async_std::{
  prelude::{FutureExt, StreamExt},
  sync::{channel, Receiver, Sender},
};
use futures::future::Future;
use std::marker::PhantomData;

/// Enum of messages we can receive from a remote client connector.
pub enum ButtplugRemoteClientConnectorMessage {
  /// Send when connection is established.
  Connected,
  /// Text version of message we received from remote server.
  Message(ButtplugSerializedMessage),
  /// Error received from remote server.
  Error(String),
  /// Connector (or remote server) itself closed the connection.
  Close(String),
}

enum StreamValue {
  NoValue,
  Incoming(ButtplugRemoteClientConnectorMessage),
  Outgoing(ButtplugClientMessageFuturePair),
}

async fn remote_connector_helper_loop<T>(
  // Takes messages from the client
  mut client_recv: Receiver<ButtplugClientMessageFuturePair>,
  // Sends messages not matched in the sorter to the client.
  client_event_sender: Sender<ButtplugClientOutMessage>,
  // Sends sorter processed messages to the connector.
  connector_input_sender: Sender<ButtplugSerializedMessage>,
  // Takes data coming in from the connector.
  mut connector_output_recv: Receiver<ButtplugRemoteClientConnectorMessage>,
) where T: ButtplugMessageSerializer<Inbound = ButtplugClientOutMessage, Outbound = ButtplugClientInMessage> + 'static {
  // Message sorter that receives messages that come in from the client, then is
  // used to match responses as they return from the server.
  let mut sorter = ClientConnectorMessageSorter::default();
  let mut serializer = T::default();
  loop {
    // We use two Options instead of an enum because we may never
    // get anything.
    let mut stream_return: StreamValue = async {
      match connector_output_recv.next().await {
        Some(msg) => StreamValue::Incoming(msg),
        None => StreamValue::NoValue,
      }
    }
    .race(async {
      match client_recv.next().await {
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
          ButtplugRemoteClientConnectorMessage::Message(serialized_msg) => {
            match serializer.deserialize(serialized_msg) {
              Ok(array) => {
                for smsg in array {
                  if !sorter.maybe_resolve_message(&smsg).await {
                    debug!("Sending event!");
                    // Send notification through event channel
                    client_event_sender.send(smsg).await;
                  } else {
                    debug!("future resolved!");
                  }
                }
              }
              Err(e) => {
                let error_str =
                  format!("Got invalid messages from remote Buttplug Server: {:?}", e);
                error!("{}", error_str);
                client_event_sender
                  .send(ButtplugClientOutMessage::Error(
                    ButtplugError::ButtplugMessageError(ButtplugMessageError::new(&error_str))
                      .into(),
                  ))
                  .await;
              }
            }
          }
          ButtplugRemoteClientConnectorMessage::Close(s) => {
            info!("Connector closing connection {}", s);
            break;
          }
          // TODO We should probably make connecting an event?
          ButtplugRemoteClientConnectorMessage::Connected => {}
          // TODO We should probably figure out what this even does?
          ButtplugRemoteClientConnectorMessage::Error(_) => {}
        }
      }
      // If we receive something from the client, register it with our sorter
      // then let the connector figure out what to do with it.
      StreamValue::Outgoing(ref mut buttplug_fut_msg) => {
        // Create future sets our message ID, so make sure this
        // happens before we send out the message.
        sorter.register_future(buttplug_fut_msg);
        let serialized_msg = serializer.serialize(vec!(buttplug_fut_msg.msg.clone()));
        connector_input_sender.send(serialized_msg).await;
      }
    }
  }
}

/// Buttplug communication helper for remote connectors
///
/// Maintaining a remote connection for a
/// [ButtplugClient][crate::client::ButtplugClient] is a complicated job.
/// Regardless of the communication type (IPC, Network, etc) used, the following
/// things are all the job of a remote connector:
///
/// - It needs to take messages from a
///   [ButtplugClient][crate::client::ButtplugClient], hand them to a
///   [ClientConnectorMessageSorter] for id setting and future resolution
///   management, then send them on to the server.
/// - It needs to take messages coming off the wire, validate and deserialize
///   them, possibly match them to waiting futures in the
///   [ClientConnectorMessageSorter], or else send them to the
///   [ButtplugClient][crate::client::ButtplugClient] as events.
/// - It needs to handle either side closing the connection at any time.
///
/// A lot of this code will look the same between all remote connectors, which
/// is where the [ButtplugRemoteClientConnectorHelper] comes in. It abstracts
/// the management of all of the above jobs, leaving the connector itself to set
/// up the methods to send/receive messages remotely. Connectors can hold an
/// instance of the [ButtplugRemoteClientConnectorHelper], and feed messages it
/// receives into the helper, while listening to the helper provided stream to
/// send things out to the wire.
///
/// # Using the Remote Client Connector Helper
///
/// The Remote Client Connector Helper is made to sit in between the
/// [ButtplugClient][crate::client::ButtplugClient]'s connector access and the
/// actual communication library being used. It has 2 channels that it
/// maintains:
///
/// - Message I/O with the [ButtplugClient][crate::client::ButtplugClient]
/// - Message I/O with whatever communication library is used.
///
/// It listens on both of these channels (using select/race), and routes
/// messages accordingly.
///
/// When building a connector that uses the helper, it's usually best to wire
/// the [ButtplugClientConnector][crate::client::ButtplugClientConnector] trait
/// methods directly to the helper. For sending/receiving, the
/// `get_remote_send()` method can be used for retreiving the channel sender,
/// which can be used by the communication library of choice to pipe its input
/// into the helper.
#[cfg(feature = "serialize_json")]
#[derive(Default)]
pub struct ButtplugRemoteClientConnectorHelper<T> {
  internal_send: Option<Sender<ButtplugClientMessageFuturePair>>,
  unused: PhantomData<T>,
}

#[cfg(feature = "serialize_json")]
unsafe impl<T> Send for ButtplugRemoteClientConnectorHelper<T> {}
#[cfg(feature = "serialize_json")]
unsafe impl<T> Sync for ButtplugRemoteClientConnectorHelper<T> {}

#[cfg(feature = "serialize_json")]
impl<T> ButtplugRemoteClientConnectorHelper<T>
  where T: ButtplugMessageSerializer<Inbound = ButtplugClientOutMessage, Outbound = ButtplugClientInMessage> + 'static 
{
  /// Returns the helper event loop future and corresponding channels.
  ///
  /// After the connector that owns this helper is actually connected, it should
  /// use this method to retreive the helper event loop future, as well as the
  /// channels required to interact with the helper event loop. The following
  /// values are returned in a tuple:
  ///
  /// - The Future itself, which needs to be run (.await'd) in the same scope
  ///   it's received in as not to violate [Pinning][std::pin].
  /// - The connector input receiver, which is where messages from the client
  ///   will arrive from, to be sent to the remote connector.
  /// - The connector output sender, which is where messages from the remote
  ///   connector should be sent for processing.
  pub fn get_event_loop_future(
    &mut self,
    // Sends messages not matched in the sorter to the client.
    client_event_sender: Sender<ButtplugClientOutMessage>,
  ) -> (impl Future, Receiver<ButtplugSerializedMessage>, Sender<ButtplugRemoteClientConnectorMessage>) 
  where T: ButtplugMessageSerializer<Inbound = ButtplugClientOutMessage, Outbound = ButtplugClientInMessage> + 'static {
    // TODO Should have this check for self.internal_send and return a result,
    // as we should not allow this to be called twice.
    let (client_send, client_recv) = channel(256);
    self.internal_send = Some(client_send);
    let (connector_input_sender, connector_input_recv) = channel(256);
    let (connector_output_sender, connector_output_recv) = channel(256);
    let fut = remote_connector_helper_loop::<T>(client_recv, client_event_sender, connector_input_sender, connector_output_recv);
    (fut, connector_input_recv, connector_output_sender)
  }

  /// Sends a message to the remote connector, via the helper's event loop
  ///
  /// After the event loop is spun up, this method will take an outgoing
  /// buttplug message and send it to the event loop, which will then forward it
  /// onto the remote connector.
  pub async fn send(
    &mut self,
    msg: &ButtplugClientInMessage,
  ) -> ButtplugInternalClientMessageResult {
    if let Some(internal_send) = &self.internal_send {
      // between tasks.
      let fut = ButtplugClientMessageFuture::default();
      internal_send
        .send(ButtplugClientMessageFuturePair::new(
          msg.clone(),
          fut.get_state_clone(),
        ))
        .await;
      fut.await
    } else {
      Err(ButtplugClientConnectorError::new(
        "Cannot send messages, connector not connected.",
      ))
    }
  }

  pub async fn close(&self) {
    // TODO We should probably be able to like, close connections, huh.
    /*
    self
      .remote_send
      .send(ButtplugRemoteClientConnectorMessage::Close(
        "Client requested close.".to_owned(),
      ))
      .await;
      */
  }
}
