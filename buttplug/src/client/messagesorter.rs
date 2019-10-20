use crate::core::messages::{ButtplugMessage, ButtplugMessageUnion};
use super::internal::ButtplugClientMessageStateShared;
use std::collections::HashMap;

pub struct ClientConnectorMessageSorter {
    future_map: HashMap<u32, ButtplugClientMessageStateShared>,
    current_id: u32,
}

impl ClientConnectorMessageSorter {
    pub fn new() -> ClientConnectorMessageSorter {
        ClientConnectorMessageSorter {
            future_map: HashMap::new(),
            current_id: 1,
        }
    }

    pub fn register_future(
        &mut self,
        msg: &mut ButtplugMessageUnion,
        state: &ButtplugClientMessageStateShared,
    ) {
        msg.set_id(self.current_id);
        self.future_map.insert(self.current_id, state.clone());
        self.current_id += 1;
    }

    pub fn maybe_resolve_message(&mut self, msg: &ButtplugMessageUnion) -> bool {
        match self.future_map.remove(&(msg.get_id())) {
            Some(_state) => {
                let mut waker_state = _state.lock().unwrap();
                waker_state.set_reply_msg(msg);
                true
            }
            None => {
                println!("Not found, may be event.");
                false
            }
        }
    }
}
