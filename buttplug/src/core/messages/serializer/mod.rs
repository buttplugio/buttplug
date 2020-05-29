mod json_serializer;

pub use json_serializer::{ButtplugClientJSONSerializer, ButtplugServerJSONSerializer};

use crate::core::errors::ButtplugError;

pub type ButtplugSerializerResult<T> = Result<T, ButtplugError>;

#[derive(Debug, PartialEq)]
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
    &mut self,
    msg: ButtplugSerializedMessage,
  ) -> ButtplugSerializerResult<Vec<Self::Inbound>>;
  fn serialize(&mut self, msg: Vec<Self::Outbound>) -> ButtplugSerializedMessage;
}
