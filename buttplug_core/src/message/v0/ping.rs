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
#[derive(
  Debug, ButtplugMessage, ButtplugMessageFinalizer, Clone, PartialEq, Eq, Serialize, Deserialize,
)]
pub struct PingV0 {
  /// Message Id, used for matching message pairs in remote connection instances.
  #[serde(rename = "Id")]
  id: u32,
}

impl Default for PingV0 {
  /// Creates a new Ping message with the given Id.
  fn default() -> Self {
    Self { id: 1 }
  }
}

impl ButtplugMessageValidator for PingV0 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}
