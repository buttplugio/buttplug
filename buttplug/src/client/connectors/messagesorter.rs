// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Handling of remote message pairing and future resolution.

use crate::{
  core::messages::{ButtplugClientInMessage, ButtplugClientOutMessage, ButtplugMessage},
  util::future::ButtplugMessageStateShared,
};
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
  future_map: HashMap<u32, ButtplugMessageStateShared>,
  current_id: u32,
}

impl ClientConnectorMessageSorter {
  pub fn register_future(
    &mut self,
    msg: &mut ButtplugClientInMessage,
    state: &ButtplugMessageStateShared,
  ) {
    msg.set_id(self.current_id);
    self.future_map.insert(self.current_id, state.clone());
    self.current_id += 1;
  }

  pub fn maybe_resolve_message(&mut self, msg: &ButtplugClientOutMessage) -> bool {
    match self.future_map.remove(&(msg.get_id())) {
      Some(_state) => {
        let mut waker_state = _state.lock().unwrap();
        waker_state.set_reply(msg.clone());
        true
      }
      None => {
        info!("Not found, may be event.");
        false
      }
    }
  }
}

impl Default for ClientConnectorMessageSorter {
  fn default() -> Self {
    Self {
      future_map: HashMap::<u32, ButtplugMessageStateShared>::new(),
      current_id: 1,
    }
  }
}
