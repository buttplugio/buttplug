use buttplug_core::message::{
  serializer::{
    json_serializer::{create_message_validator, deserialize_to_message, vec_to_protocol_json},
    ButtplugMessageSerializer,
    ButtplugSerializedMessage,
    ButtplugSerializerError,
  },
  ButtplugClientMessageV4,
  ButtplugMessage,
  ButtplugMessageFinalizer,
  ButtplugServerMessageV4,
};
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
  type Inbound = ButtplugServerMessageV4;
  type Outbound = ButtplugClientMessageV4;

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

#[cfg(test)]
mod test {
  use super::*;
  use buttplug_core::message::{
    RequestServerInfoV4,
    BUTTPLUG_CURRENT_API_MAJOR_VERSION,
    BUTTPLUG_CURRENT_API_MINOR_VERSION,
  };

  #[test]
  fn test_client_incorrect_messages() {
    let incorrect_incoming_messages = vec![
      // Not valid JSON
      "not a json message",
      // Valid json object but no contents
      "{}",
      // Valid json but not an object
      "[]",
      // Not a message type
      "[{\"NotAMessage\":{}}]",
      // Valid json and message type but not in correct format
      "[{\"Ok\":[]}]",
      // Valid json and message type but not in correct format
      "[{\"Ok\":{}}]",
      // Valid json and message type but not an array.
      "{\"Ok\":{\"Id\":0}}",
      // Valid json and message type but not an array.
      "[{\"Ok\":{\"Id\":0}}]",
      // Valid json and message type but with extra content
      "[{\"Ok\":{\"NotAField\":\"NotAValue\",\"Id\":1}}]",
    ];
    let serializer = ButtplugClientJSONSerializer::default();
    let _ = serializer.serialize(&vec![RequestServerInfoV4::new(
      "test client",
      BUTTPLUG_CURRENT_API_MAJOR_VERSION,
      BUTTPLUG_CURRENT_API_MINOR_VERSION,
    )
    .into()]);
    for msg in incorrect_incoming_messages {
      let res = serializer.deserialize(&ButtplugSerializedMessage::Text(msg.to_owned()));
      assert!(res.is_err(), "{} should be an error", msg);
      if let Err(ButtplugSerializerError::MessageSpecVersionNotReceived) = res {
        assert!(false, "Wrong error!");
      }
    }
  }
}
