use buttplug_core::message::{
    serializer::{
      json_serializer::{create_message_validator, deserialize_to_message, vec_to_protocol_json},
      ButtplugMessageSerializer,
      ButtplugSerializedMessage,
      ButtplugSerializerError,
    },
    ButtplugMessage,
    ButtplugMessageFinalizer,
  };
use buttplug_server::message::{ButtplugClientMessageV3, ButtplugServerMessageV3};

use jsonschema::Validator;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub struct ButtplugClientJSONSerializerImpl {
  validator: Validator,
}

impl Default for ButtplugClientJSONSerializerImpl {
  fn default() -> Self {
    Self {
      validator: create_message_validator(),
    }
  }
}

impl ButtplugClientJSONSerializerImpl {
  pub fn deserialize<T>(
    &self,
    msg: &ButtplugSerializedMessage,
  ) -> Result<Vec<T>, ButtplugSerializerError>
  where
    T: serde::de::DeserializeOwned + ButtplugMessageFinalizer + Clone + Debug,
  {
    if let ButtplugSerializedMessage::Text(text_msg) = msg {
      deserialize_to_message::<T>(Some(&self.validator), text_msg)
    } else {
      Err(ButtplugSerializerError::BinaryDeserializationError)
    }
  }

  pub fn serialize<T>(&self, msg: &[T]) -> ButtplugSerializedMessage
  where
    T: ButtplugMessage + Serialize + Deserialize<'static>,
  {
    ButtplugSerializedMessage::Text(vec_to_protocol_json(msg))
  }
}

#[derive(Default)]
pub struct ButtplugClientJSONSerializer {
  serializer_impl: ButtplugClientJSONSerializerImpl,
}

impl ButtplugMessageSerializer for ButtplugClientJSONSerializer {
  type Inbound = ButtplugServerMessageV3;
  type Outbound = ButtplugClientMessageV3;

  fn deserialize(
    &self,
    msg: &ButtplugSerializedMessage,
  ) -> Result<Vec<Self::Inbound>, ButtplugSerializerError> {
    self.serializer_impl.deserialize(msg)
  }

  fn serialize(&self, msg: &[Self::Outbound]) -> ButtplugSerializedMessage {
    self.serializer_impl.serialize(msg)
  }
}
