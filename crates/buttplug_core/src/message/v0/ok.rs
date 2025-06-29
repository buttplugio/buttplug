// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::{
  ButtplugMessage,
  ButtplugMessageError,
  ButtplugMessageFinalizer,
  ButtplugMessageValidator,
};
use serde::{Deserialize, Serialize};

/// Ok message, signifying successful response to a command. [Spec link](https://buttplug-spec.docs.buttplug.io/status.html#ok).
#[derive(
  Debug, PartialEq, Eq, ButtplugMessage, ButtplugMessageFinalizer, Clone, Serialize, Deserialize,
)]
pub struct OkV0 {
  /// Message Id, used for matching message pairs in remote connection instances.
  #[serde(rename = "Id")]
  id: u32,
}

impl OkV0 {
  /// Creates a new Ok message with the given Id.
  pub fn new(id: u32) -> Self {
    Self { id }
  }
}

impl Default for OkV0 {
  fn default() -> Self {
    Self { id: 1 }
  }
}

impl ButtplugMessageValidator for OkV0 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}

#[cfg(test)]
mod test {
  use crate::message::{ButtplugServerMessageCurrent, OkV0};

  const OK_STR: &str = "{\"Ok\":{\"Id\":0}}";

  #[test]
  fn test_ok_serialize() {
    let ok = ButtplugServerMessageCurrent::Ok(OkV0::new(0));
    let js = serde_json::to_string(&ok).expect("Infallible serialization");
    assert_eq!(OK_STR, js);
  }

  #[test]
  fn test_ok_deserialize() {
    let union: ButtplugServerMessageCurrent =
      serde_json::from_str(OK_STR).expect("Infallible deserialization");
    assert_eq!(ButtplugServerMessageCurrent::Ok(OkV0::new(0)), union);
  }
}
