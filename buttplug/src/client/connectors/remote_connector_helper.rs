// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{
  ButtplugClientConnectorError, ButtplugClientMessageFuture, ButtplugClientMessageFuturePair,
  ButtplugInternalClientMessageResult, ClientConnectorMessageSorter,
};
use crate::core::{
  errors::{ButtplugError, ButtplugMessageError},
  messages::{create_message_validator, ButtplugClientInMessage, ButtplugClientOutMessage},
};
use async_std::{
  prelude::{FutureExt, StreamExt},
  sync::{channel, Receiver, Sender},
};
use futures::future::Future;

/// Enum of messages we can receive from a remote client connector.
pub enum ButtplugRemoteClientConnectorMessage {
  /// Send when connection is established.
  Connected,
  /// Text version of message we received from remote server.
  Text(String),
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

async fn remote_connector_helper_loop(
  // Takes messages from the client
  client_recv: Receiver<ButtplugClientMessageFuturePair>,
  // Sends messages not matched in the sorter to the client.
  client_event_sender: Sender<ButtplugClientOutMessage>,
  // Sends sorter processed messages to the connector.
  connector_input_sender: Sender<ButtplugClientInMessage>,
  // Takes data coming in from the connector.
  connector_output_recv: Receiver<ButtplugRemoteClientConnectorMessage>,
) {
  // Message sorter that receives messages that come in from the client, then is
  // used to match responses as they return from the server.
  let mut sorter = ClientConnectorMessageSorter::default();

  // JSON validator for checking messages that come in from the server.
  let message_validator = create_message_validator();

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
      StreamValue::NoValue => break,
      StreamValue::Incoming(remote_msg) => {
        match remote_msg {
          ButtplugRemoteClientConnectorMessage::Text(t) => {
            match message_validator.validate(&t.clone()) {
              Ok(_) => {
                let array: Vec<ButtplugClientOutMessage> =
                  serde_json::from_str(&t.clone()).unwrap();
                for smsg in array {
                  if !sorter.maybe_resolve_message(&smsg).await {
                    debug!("Sending event!");
                    // Send notification through event channel
                    client_event_sender.send(smsg).await;
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
          ButtplugRemoteClientConnectorMessage::Connected => {}
          ButtplugRemoteClientConnectorMessage::Error(_) => {}
        }
      }
      StreamValue::Outgoing(ref mut buttplug_fut_msg) => {
        // Create future sets our message ID, so make sure this
        // happens before we send out the message.
        sorter.register_future(buttplug_fut_msg);
        connector_input_sender.send(buttplug_fut_msg.msg.clone());
      }
    }
  }
}

/// Buttplug communication helper for remote connectors
///
/// Maintaining a remote connection for a [ButtplugClient] is a complicated job.
/// Regardless of the communication type (IPC, Network, etc) used, the following
/// things are all the job of a remote connector:
///
/// - It needs to take messages from a [ButtplugClient], hand them to a
///   [ClientConnectorMessageSorter] for id setting and future resolution
///   management, then send them on to the server.
/// - It needs to take messages coming off the wire, validate and deserialize
///   them, possibly match them to waiting futures in the
///   [ClientConnectorMessageSorter], or else send them to the [ButtplugClient]
///   as events.
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
/// [ButtplugClient]'s connector access and the actual communication library
/// being used. It has 2 channels that it maintains:
///
/// - Message I/O with the [ButtplugClient]
/// - Message I/O with whatever communication library is used.
///
/// It listens on both of these channels (using select/race), and routes
/// messages accordingly.
///
/// When building a connector that uses the helper, it's usually best to wire
/// the [ButtplugClientConnector] trait methods directly to the helper. For
/// sending/receiving, the `get_remote_send()` method can be used for retreiving
/// the channel sender, which can be used by the communication library of choice
/// to pipe its input into the helper.
#[cfg(feature = "serialize_json")]
#[derive(Default)]
pub struct ButtplugRemoteClientConnectorHelper {
  internal_send: Option<Sender<ButtplugClientMessageFuturePair>>,
}

#[cfg(feature = "serialize_json")]
unsafe impl Send for ButtplugRemoteClientConnectorHelper {}
#[cfg(feature = "serialize_json")]
unsafe impl Sync for ButtplugRemoteClientConnectorHelper {}

#[cfg(feature = "serialize_json")]
impl ButtplugRemoteClientConnectorHelper {
  pub fn get_event_loop_future(
    &mut self,
    // Sends messages not matched in the sorter to the client.
    client_event_sender: Sender<ButtplugClientOutMessage>,
  ) -> (impl Future, Receiver<ButtplugClientInMessage>, Sender<ButtplugRemoteClientConnectorMessage>) {
    let (client_send, client_recv) = channel(256);
    self.internal_send = client_send;
    let (connector_input_sender, connector_input_recv) = channel(256);
    let (connector_output_sender, connector_output_recv) = channel(256);
    let fut = remote_connector_helper_loop(client_recv, client_event_sender, connector_input_sender, connector_output_recv);
    (fut, connector_input_recv, connector_output_sender)
  }

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
