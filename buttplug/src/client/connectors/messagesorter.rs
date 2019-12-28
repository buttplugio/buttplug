// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Handling of remote message pairing and future resolution.

use crate::{
    core::messages::{ButtplugMessage, ButtplugMessageUnion},
    util::future::ButtplugMessageStateShared,
};
use std::collections::HashMap;

pub struct ClientConnectorMessageSorter {
    future_map: HashMap<u32, ButtplugMessageStateShared>,
    current_id: u32,
}

impl ClientConnectorMessageSorter {
    pub fn register_future(
        &mut self,
        msg: &mut ButtplugMessageUnion,
        state: &ButtplugMessageStateShared,
    ) {
        msg.set_id(self.current_id);
        self.future_map.insert(self.current_id, state.clone());
        self.current_id += 1;
    }

    pub fn maybe_resolve_message(&mut self, msg: &ButtplugMessageUnion) -> bool {
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
            future_map: HashMap::new(),
            current_id: 1,
        }
    }
}
