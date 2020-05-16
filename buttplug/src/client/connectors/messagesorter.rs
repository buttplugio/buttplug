// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Handling of remote message pairing and future resolution.

use crate::{
  core::messages::{ButtplugClientInMessage, ButtplugClientOutMessage, ButtplugMessage},
};
use super::super::ButtplugClientMessageStateShared;
use std::collections::HashMap;

/// Handling of message sorting for client connectors.
///
/// In order to create connections to remote systems, we need a way to maintain
/// message coherence. We expect that whenever a client sends the server a
/// message, the server will always respond. 
///
/// In the embedded case, where the client and server are in the same process,
/// we can simply use execution flow to match the client message and server
/// response. However, when going over IPC or network, we have to wait to hear
/// back from the server. To match the outgoing client message with the incoming
/// server message in the remote case, we use the Id field of
/// [ButtplugMessage]. The client's message will have a server response with a
/// matching index. Any message that comes from the server without an
/// originating client message (DeviceAdded, Log, etc...) will have an index of
/// 0.
///
/// The ClientConnectionMessageSorter does two things to facilitate this
/// matching:
///
/// - Keeps track of the current message index, as a u32
/// - Manages a map of indexes to resolvable futures.
///
/// Whenever a remote connector sends a message, it first puts it through its
/// ClientConnectorMessageSorter to fill in the message index. Similarly, when a
/// message is received, it comes through the sorter, with one of 3 outcomes:
///
/// - If there is a future with matching index waiting on a response, it
///   resolves that future using the incoming message
/// - If the message index is 0, the message is emitted as an event.
/// - If the message index is not zero but there is no future waiting, the
///   message is dropped and an error is emitted.
///
pub struct ClientConnectorMessageSorter {
  /// Map of message Ids to their related future.
  ///
  /// This is where we store message Ids that are waiting for a return from the
  /// server. Once we get back a response with a matching Id, we remove the
  /// entry from this map, and use the waker to complete the future with the
  /// received response.
  future_map: HashMap<u32, ButtplugClientMessageStateShared>,

  /// Message Id counter
  ///
  /// Every time we add a message to the future_map, we need it to have a unique
  /// Id. We assume that unsigned 2^32 will be enough (Buttplug isn't THAT
  /// chatty), and use it as a monotonically increasing counter for setting Ids.
  current_id: u32,
}

impl ClientConnectorMessageSorter {
  /// Registers a future to be resolved when we receive a response.
  ///
  /// Given a message and its related future, set the message's id, and match
  /// that id with the future to be resolved when we get a response back.
  ///
  /// # Arguments
  ///
  /// `msg` - Message that needs a response from the server. We'll used the
  /// message's Id to match to match to the response and complete the
  /// appropriate future.
  ///
  /// `state` - Waker for the future we'll need to resolve when the message
  /// response is received.
  pub fn register_future(
    &mut self,
    msg: &mut ButtplugClientInMessage,
    state: &ButtplugClientMessageStateShared,
  ) {
    trace!("Setting message id to {}", self.current_id);
    msg.set_id(self.current_id);
    self.future_map.insert(self.current_id, state.clone());
    self.current_id += 1;
  }

  /// Given a response message from the server, resolve related message if we have one.
  ///
  /// Returns true if the message was resolved to a future, otherwise returns false.
  ///
  /// # Arguments
  ///
  /// - `msg` - Message from the server, may or may not be a response.
  pub async fn maybe_resolve_message(&mut self, msg: &ButtplugClientOutMessage) -> bool {
    let id = msg.get_id();
    trace!("Trying to resolve message future for id {}.", id);
    match self.future_map.remove(&id) {
      Some(_state) => {
        trace!("Resolved id {} to a future.", id);
        let mut waker_state = _state.try_lock().expect("Future locks should never be in contention");
        waker_state.set_reply(msg.clone());
        true
      }
      None => {
        trace!("Message id {} not found, considering it an event.", id);
        false
      }
    }
  }
}

impl Default for ClientConnectorMessageSorter {
  /// Create a default implementation of the ClientConnectorMessageSorter
  ///
  /// Sets the current_id to 1, since we can't send message id of 0 (0 is
  /// reserved for system incoming messages).
  fn default() -> Self {
    Self {
      future_map: HashMap::<u32, ButtplugClientMessageStateShared>::new(),
      current_id: 1,
    }
  }
}
