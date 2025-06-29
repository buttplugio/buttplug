// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Message de/serialization handling
pub mod json_serializer;

use serde::{Deserialize, Serialize};
use thiserror::Error;
pub type ButtplugSerializerResult<T> = Result<T, ButtplugSerializerError>;

#[derive(Debug, Error, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ButtplugSerializerError {
  // jsonschema hands back a vector of errors that isn't easy to encase, so we just
  // turn it into a big string and pass that back.
  #[error("JSON Schema Validation Error: {0}")]
  JsonValidatorError(String),
  /// Serialization error.
  #[error("Cannot serialize to JSON: {0}")]
  JsonSerializerError(String),
  #[error("Cannot deserialize binary in a text handler")]
  BinaryDeserializationError,
  #[error("Cannot deserialize text in a binary handler.")]
  TextDeserializationError,
  #[error("Message version not received, can't figure out which spec version to de/serialize to.")]
  MessageSpecVersionNotReceived,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ButtplugSerializedMessage {
  Text(String),
  Binary(Vec<u8>),
}

impl From<String> for ButtplugSerializedMessage {
  fn from(msg: String) -> Self {
    ButtplugSerializedMessage::Text(msg)
  }
}

impl From<Vec<u8>> for ButtplugSerializedMessage {
  fn from(msg: Vec<u8>) -> Self {
    ButtplugSerializedMessage::Binary(msg)
  }
}

pub trait ButtplugMessageSerializer: Default + Sync + Send {
  type Inbound;
  type Outbound;
  fn deserialize(
    &self,
    msg: &ButtplugSerializedMessage,
  ) -> ButtplugSerializerResult<Vec<Self::Inbound>>;
  fn serialize(&self, msg: &[Self::Outbound]) -> ButtplugSerializedMessage;
}
