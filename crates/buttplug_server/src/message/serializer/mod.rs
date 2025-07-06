use buttplug_core::{
  errors::{ButtplugError, ButtplugHandshakeError, ButtplugMessageError},
  message::{
    self,
    serializer::{
      json_serializer::{
        create_message_validator,
        deserialize_to_message,
        msg_to_protocol_json,
        vec_to_protocol_json,
      },
      ButtplugMessageSerializer,
      ButtplugSerializedMessage,
      ButtplugSerializerError,
    },
    ButtplugClientMessageV4,
    ButtplugMessageFinalizer,
    ButtplugMessageSpecVersion,
    ButtplugServerMessageCurrent,
    ButtplugServerMessageV4,
  },
};
use jsonschema::Validator;
use once_cell::sync::OnceCell;
use serde::Deserialize;

use super::{
  ButtplugClientMessageV0,
  ButtplugClientMessageV1,
  ButtplugClientMessageV2,
  ButtplugClientMessageV3,
  ButtplugClientMessageVariant,
  ButtplugServerMessageV0,
  ButtplugServerMessageV1,
  ButtplugServerMessageV2,
  ButtplugServerMessageV3,
  ButtplugServerMessageVariant,
};

#[derive(Deserialize, ButtplugMessageFinalizer, Clone, Debug)]
struct RequestServerInfoMessage {
  #[serde(rename = "RequestServerInfo")]
  rsi: RequestServerInfoVersion,
}

#[derive(Deserialize, ButtplugMessageFinalizer, Clone, Debug)]
struct RequestServerInfoVersion {
  #[serde(rename = "Id")]
  _id: u32,
  #[serde(rename = "ClientName")]
  _client_name: String,
  #[serde(default, rename = "MessageVersion")]
  message_version: Option<u32>,
  #[serde(default, rename = "ProtocolVersionMajor")]
  api_major_version: Option<u32>,
}

pub struct ButtplugServerJSONSerializer {
  pub(super) message_version: OnceCell<message::ButtplugMessageSpecVersion>,
  validator: Validator,
}

impl Default for ButtplugServerJSONSerializer {
  fn default() -> Self {
    Self {
      message_version: OnceCell::new(),
      validator: create_message_validator(),
    }
  }
}

impl ButtplugServerJSONSerializer {
  pub fn force_message_version(&self, version: &ButtplugMessageSpecVersion) {
    self
      .message_version
      .set(*version)
      .expect("This should only ever be called once.");
  }
}

impl ButtplugMessageSerializer for ButtplugServerJSONSerializer {
  type Inbound = ButtplugClientMessageVariant;
  type Outbound = ButtplugServerMessageVariant;

  fn deserialize(
    &self,
    serialized_msg: &ButtplugSerializedMessage,
  ) -> Result<Vec<ButtplugClientMessageVariant>, ButtplugSerializerError> {
    let msg = if let ButtplugSerializedMessage::Text(text_msg) = serialized_msg {
      text_msg
    } else {
      return Err(ButtplugSerializerError::BinaryDeserializationError);
    };

    if let Some(version) = self.message_version.get() {
      return Ok(match version {
        ButtplugMessageSpecVersion::Version0 => {
          deserialize_to_message::<ButtplugClientMessageV0>(Some(&self.validator), msg)?
            .iter()
            .cloned()
            .map(|m| m.into())
            .collect()
        }
        ButtplugMessageSpecVersion::Version1 => {
          deserialize_to_message::<ButtplugClientMessageV1>(Some(&self.validator), msg)?
            .iter()
            .cloned()
            .map(|m| m.into())
            .collect()
        }
        ButtplugMessageSpecVersion::Version2 => {
          deserialize_to_message::<ButtplugClientMessageV2>(Some(&self.validator), msg)?
            .iter()
            .cloned()
            .map(|m| m.into())
            .collect()
        }
        ButtplugMessageSpecVersion::Version3 => {
          deserialize_to_message::<ButtplugClientMessageV3>(Some(&self.validator), msg)?
            .iter()
            .cloned()
            .map(|m| m.into())
            .collect()
        }
        ButtplugMessageSpecVersion::Version4 => {
          deserialize_to_message::<ButtplugClientMessageV4>(Some(&self.validator), msg)?
            .iter()
            .cloned()
            .map(|m| m.into())
            .collect()
        }
      });
    }
    // If we don't have a message version yet, we need to parse this as a RequestServerInfo message
    // to get the version. As of v4, RequestServerInfo is of a different layout than RSI v0-v3,
    // therefore we need to step through versions for compatibility sake.
    info!("{:?}", msg);
    let msg_version =
      if let Ok(msg_union) = deserialize_to_message::<RequestServerInfoMessage>(None, msg) {
        info!("PARSING {:?}", msg_union);
        if msg_union.is_empty() {
          Err(ButtplugSerializerError::MessageSpecVersionNotReceived)
        } else if let Some(v) = msg_union[0].rsi.api_major_version {
          ButtplugMessageSpecVersion::try_from(v as i32)
            .map_err(|_| ButtplugSerializerError::MessageSpecVersionNotReceived)
        } else if let Some(v) = msg_union[0].rsi.message_version {
          ButtplugMessageSpecVersion::try_from(v as i32)
            .map_err(|_| ButtplugSerializerError::MessageSpecVersionNotReceived)
        } else {
          Ok(ButtplugMessageSpecVersion::Version0)
        }
      } else {
        info!("NOT EVEN PARSING");
        Err(ButtplugSerializerError::MessageSpecVersionNotReceived)
      }?;

    info!("Setting JSON Wrapper message version to {}", msg_version);
    self
      .message_version
      .set(msg_version)
      .expect("This should only ever be called once.");
    // Now that we know our version, parse the message again.
    self.deserialize(serialized_msg)
  }

  fn serialize(&self, msgs: &[ButtplugServerMessageVariant]) -> ButtplugSerializedMessage {
    if let Some(version) = self.message_version.get() {
      ButtplugSerializedMessage::Text(match version {
        ButtplugMessageSpecVersion::Version0 => {
          let msg_vec: Vec<ButtplugServerMessageV0> = msgs
            .iter()
            .map(|msg| match msg {
              ButtplugServerMessageVariant::V0(msgv0) => msgv0.clone(),
              _ => ButtplugServerMessageV0::Error(message::ErrorV0::from(ButtplugError::from(
                ButtplugMessageError::MessageConversionError(format!(
                  "Message {msg:?} not in Spec V0! This is a server bug."
                )),
              ))),
            })
            .collect();
          vec_to_protocol_json(&msg_vec)
        }
        ButtplugMessageSpecVersion::Version1 => {
          let msg_vec: Vec<ButtplugServerMessageV1> = msgs
            .iter()
            .map(|msg| match msg {
              ButtplugServerMessageVariant::V1(msgv1) => msgv1.clone(),
              _ => ButtplugServerMessageV1::Error(message::ErrorV0::from(ButtplugError::from(
                ButtplugMessageError::MessageConversionError(format!(
                  "Message {msg:?} not in Spec V1! This is a server bug."
                )),
              ))),
            })
            .collect();
          vec_to_protocol_json(&msg_vec)
        }
        ButtplugMessageSpecVersion::Version2 => {
          let msg_vec: Vec<ButtplugServerMessageV2> = msgs
            .iter()
            .map(|msg| match msg {
              ButtplugServerMessageVariant::V2(msgv2) => msgv2.clone(),
              _ => ButtplugServerMessageV2::Error(message::ErrorV0::from(ButtplugError::from(
                ButtplugMessageError::MessageConversionError(format!(
                  "Message {msg:?} not in Spec V2! This is a server bug."
                )),
              ))),
            })
            .collect();
          vec_to_protocol_json(&msg_vec)
        }
        ButtplugMessageSpecVersion::Version3 => {
          let msg_vec: Vec<ButtplugServerMessageV3> = msgs
            .iter()
            .map(|msg| match msg {
              ButtplugServerMessageVariant::V3(msgv3) => msgv3.clone(),
              _ => ButtplugServerMessageV3::Error(message::ErrorV0::from(ButtplugError::from(
                ButtplugMessageError::MessageConversionError(format!(
                  "Message {msg:?} not in Spec V3! This is a server bug."
                )),
              ))),
            })
            .collect();
          vec_to_protocol_json(&msg_vec)
        }
        ButtplugMessageSpecVersion::Version4 => {
          let msg_vec: Vec<ButtplugServerMessageV4> = msgs
            .iter()
            .map(|msg| match msg {
              ButtplugServerMessageVariant::V4(msgv4) => msgv4.clone(),
              _ => ButtplugServerMessageV4::Error(message::ErrorV0::from(ButtplugError::from(
                ButtplugMessageError::MessageConversionError(format!(
                  "Message {msg:?} not in Spec V4! This is a server bug."
                )),
              ))),
            })
            .collect();
          vec_to_protocol_json(&msg_vec)
        }
      })
    } else {
      // If we don't even have enough info to know which message
      // version to convert to, consider this a handshake error.
      ButtplugSerializedMessage::Text(msg_to_protocol_json(ButtplugServerMessageCurrent::Error(
        ButtplugError::from(ButtplugHandshakeError::RequestServerInfoExpected).into(),
      )))
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_correct_message_version() {
    let json = r#"[{
            "RequestServerInfo": {
                "Id": 1,
                "ClientName": "Test Client",
                "ProtocolVersionMajor": 4,
                "ProtocolVersionMinor": 0
            }
        }]"#;
    let serializer = ButtplugServerJSONSerializer::default();
    serializer
      .deserialize(&ButtplugSerializedMessage::Text(json.to_owned()))
      .expect("Infallible deserialization");
    assert_eq!(
      *serializer.message_version.get().unwrap(),
      ButtplugMessageSpecVersion::Version4
    );
  }

  #[test]
  fn test_wrong_message_version() {
    let json = r#"[{
      "RequestServerInfo": {
          "Id": 1,
          "ClientName": "Test Client",
          "ProtocolVersionMajor": 100,
          "ProtocolVersionMinor": 0
      }
    }]"#;
    let serializer = ButtplugServerJSONSerializer::default();
    let msg = serializer.deserialize(&ButtplugSerializedMessage::Text(json.to_owned()));
    info!("{:?}", msg);
    assert!(msg.is_err());
  }

  #[test]
  fn test_message_array() {
    let json = r#"[
      {
        "RequestServerInfo": {
          "Id": 1,
          "ClientName": "Test Client",
          "ProtocolVersionMajor": 4,
          "ProtocolVersionMinor": 0
        }
      },
      {
        "RequestServerInfo": {
            "Id": 1,
            "ClientName": "Test Client",
            "ProtocolVersionMajor": 4,
            "ProtocolVersionMinor": 0
        }
      },
      {
        "RequestServerInfo": {
          "Id": 1,
          "ClientName": "Test Client",
          "ProtocolVersionMajor": 4,
          "ProtocolVersionMinor": 0
        }
      }
    ]"#;
    let serializer = ButtplugServerJSONSerializer::default();
    let messages = serializer
      .deserialize(&ButtplugSerializedMessage::Text(json.to_owned()))
      .expect("Infallible deserialization");
    assert_eq!(messages.len(), 3);
  }

  #[test]
  fn test_streamed_message_array() {
    let json = r#"[
        {
          "RequestServerInfo": {
            "Id": 1,
            "ClientName": "Test Client",
            "ProtocolVersionMajor": 4,
            "ProtocolVersionMinor": 0
          }
        }]
        [{
          "RequestServerInfo": {
            "Id": 1,
            "ClientName": "Test Client",
            "ProtocolVersionMajor": 4,
            "ProtocolVersionMinor": 0
          }
        }]
        [{
          "RequestServerInfo": {
            "Id": 1,
            "ClientName": "Test Client",
            "ProtocolVersionMajor": 4,
            "ProtocolVersionMinor": 0
          }
        }]
    "#;
    let serializer = ButtplugServerJSONSerializer::default();
    let messages = serializer
      .deserialize(&ButtplugSerializedMessage::Text(json.to_owned()))
      .expect("Infallible deserialization");
    assert_eq!(messages.len(), 3);
  }

  #[test]
  fn test_invalid_streamed_message_array() {
    // Missing a } in the second message.
    let json = r#"[
          "RequestServerInfo": {
            "Id": 1,
            "ClientName": "Test Client",
            "ProtocolVersionMajor": 4,
            "ProtocolVersionMinor": 0
          }
        }]
        [{
          "RequestServerInfo": {
            "Id": 1,
            "ClientName": "Test Client",
            "ProtocolVersionMajor": 4,
            "ProtocolVersionMinor": 0
        }]
        [{
          "RequestServerInfo": {
            "Id": 1,
            "ClientName": "Test Client",
            "ProtocolVersionMajor": 4,
            "ProtocolVersionMinor": 0
          }
        }]
    "#;
    let serializer = ButtplugServerJSONSerializer::default();
    assert!(matches!(
      serializer.deserialize(&ButtplugSerializedMessage::Text(json.to_owned())),
      Err(_)
    ));
  }
}
