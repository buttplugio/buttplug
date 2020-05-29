// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Handling of remote message pairing and future resolution.

use crate::{
  client::{ButtplugClientMessageFuturePair, ButtplugClientMessageStateShared},
  core::messages::{ButtplugCurrentSpecServerMessage, ButtplugMessage},
};
use std::collections::HashMap;

/// Message sorting and pairing for remote client connectors.
///
/// In order to create connections to remote systems, we need a way to maintain
/// message coherence. We expect that whenever a client sends the server a
/// request message, the server will always send back a response message.
///
/// In the [embedded][crate::connector::ButtplugInProcessClientConnector] case,
/// where the client and server are in the same process, we can simply use
/// execution flow to match the client message and server response. However,
/// when going over IPC or network, we have to wait to hear back from the
/// server. To match the outgoing client request message with the incoming
/// server response message in the remote case, we use the `id` field of
/// [ButtplugMessage]. The client's request message will have a server response
/// with a matching index. Any message that comes from the server without an
/// originating client message
/// ([DeviceAdded][crate::core::messages::DeviceAdded],
/// [Log][crate::core::messages::Log], etc...) will have an `id` of 0 and is
/// considered an *event*, meaning something happened on the server that was not
/// directly tied to a client request.
///
/// The ClientConnectionMessageSorter does two things to facilitate this
/// matching:
///
/// - Creates and keeps track of the current message `id`, as a [u32]
/// - Manages a [HashMap] of indexes to resolvable futures.
///
/// Whenever a remote connector sends a [ButtplugMessage], it first puts it
/// through its [ClientConnectorMessageSorter] to fill in the message `id`.
/// Similarly, when a [ButtplugMessage] is received, it comes through the
/// sorter, with one of 3 outcomes:
///
/// - If there is a future with matching `id` waiting on a response, it resolves
///   that future using the incoming message
/// - If the message `id` is 0, the message is emitted as an *event*.
/// - If the message `id` is not zero but there is no future waiting, the
///   message is dropped and an error is emitted.
///
pub struct ClientMessageSorter {
  /// Map of message `id`s to their related future.
  ///
  /// This is where we store message `id`s that are waiting for a return from
  /// the server. Once we get back a response with a matching `id`, we remove
  /// the entry from this map, and use the waker to complete the future with the
  /// received response message.
  future_map: HashMap<u32, ButtplugClientMessageStateShared>,

  /// Message `id` counter
  ///
  /// Every time we add a message to the future_map, we need it to have a unique
  /// `id`. We assume that unsigned 2^32 will be enough (Buttplug isn't THAT
  /// chatty), and use it as a monotonically increasing counter for setting `id`s.
  current_id: u32,
}

impl ClientMessageSorter {
  /// Registers a future to be resolved when we receive a response.
  ///
  /// Given a message and its related future, set the message's `id`, and match
  /// that id with the future to be resolved when we get a response back.
  pub fn register_future(&mut self, msg_fut: &mut ButtplugClientMessageFuturePair) {
    trace!("Setting message id to {}", self.current_id);
    msg_fut.msg.set_id(self.current_id);
    self
      .future_map
      .insert(self.current_id, msg_fut.waker.clone());
    self.current_id += 1;
  }

  /// Given a response message from the server, resolve related future if we
  /// have one.
  ///
  /// Returns true if the response message was resolved to a future via matching
  /// `id`, otherwise returns false. False returns mean the message should be
  /// considered as an *event*.
  pub async fn maybe_resolve_message(&mut self, msg: &ButtplugCurrentSpecServerMessage) -> bool {
    let id = msg.get_id();
    trace!("Trying to resolve message future for id {}.", id);
    match self.future_map.remove(&id) {
      Some(mut _state) => {
        trace!("Resolved id {} to a future.", id);
        _state.set_reply(Ok(msg.clone()));
        true
      }
      None => {
        trace!("Message id {} not found, considering it an event.", id);
        false
      }
    }
  }
}

impl Default for ClientMessageSorter {
  /// Create a default implementation of the ClientConnectorMessageSorter
  ///
  /// Sets the current_id to 1, since as a client we can't send message `id` of
  /// 0 (0 is reserved for system incoming messages).
  fn default() -> Self {
    Self {
      future_map: HashMap::<u32, ButtplugClientMessageStateShared>::new(),
      current_id: 1,
    }
  }
}
