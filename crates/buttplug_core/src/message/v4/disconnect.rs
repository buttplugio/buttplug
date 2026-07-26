// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::simple_client_message;

simple_client_message!(DisconnectV4);

#[cfg(test)]
mod test {
  use super::DisconnectV4;
  use crate::message::{ButtplugClientMessageV4, ButtplugMessage, ButtplugMessageValidator};

  const DISCONNECT_STR: &str = "{\"Disconnect\":{\"Id\":42}}";

  #[test]
  fn test_disconnect_serialize() {
    let mut msg = DisconnectV4::default();
    msg.set_id(42);
    let wrapped = ButtplugClientMessageV4::Disconnect(msg);
    let js = serde_json::to_string(&wrapped).expect("Infallible serialization");
    assert_eq!(DISCONNECT_STR, js);
  }

  #[test]
  fn test_disconnect_deserialize() {
    let wrapped: ButtplugClientMessageV4 =
      serde_json::from_str(DISCONNECT_STR).expect("Valid JSON");
    match wrapped {
      ButtplugClientMessageV4::Disconnect(msg) => {
        assert_eq!(msg.id(), 42);
        assert!(msg.is_valid().is_ok());
      }
      _ => panic!("Expected Disconnect variant"),
    }
  }

  #[test]
  fn test_disconnect_rejects_system_id() {
    let mut msg = DisconnectV4::default();
    msg.set_id(0);
    assert!(msg.is_valid().is_err());
  }
}
