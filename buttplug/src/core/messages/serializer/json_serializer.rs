use super::{ButtplugMessageSerializer, ButtplugSerializedMessage};
use crate::{
  core::{
    errors::{ButtplugError, ButtplugHandshakeError, ButtplugMessageError},
    messages::{
      self,
      ButtplugInMessage,
      ButtplugMessage,
      ButtplugMessageSpecVersion,
      ButtplugOutMessage,
      ButtplugSpecV0InMessage,
      ButtplugSpecV0OutMessage,
      ButtplugSpecV1InMessage,
      ButtplugSpecV1OutMessage,
      ButtplugSpecV2InMessage,
      ButtplugSpecV2OutMessage,
      ButtplugClientOutMessage,
      ButtplugClientInMessage,
    },
  },
  util::json::JSONValidator,
};
use std::convert::TryFrom;
use serde::{Deserialize, Serialize};

static MESSAGE_JSON_SCHEMA: &str =
  include_str!("../../../../dependencies/buttplug-schema/schema/buttplug-schema.json");

/// Creates a [Valico][valico] validator using the built in buttplug message schema.
pub fn create_message_validator() -> JSONValidator {
  JSONValidator::new(MESSAGE_JSON_SCHEMA)
}
pub struct ButtplugServerJSONSerializer {
  pub(super) message_version: Option<messages::ButtplugMessageSpecVersion>,
  validator: JSONValidator
}

impl Default for ButtplugServerJSONSerializer {
  fn default() -> Self {
    Self {
      message_version: None,
      validator: create_message_validator()
    }
  }
}

/// Returns the message as a string in Buttplug JSON Protocol format.
pub fn msg_to_protocol_json<T>(msg: T) -> String where T: ButtplugMessage + Serialize + Deserialize<'static>
{
  serde_json::to_string(&[&msg]).unwrap()
}

pub fn vec_to_protocol_json<T>(msg: Vec<T>) -> String where T: ButtplugMessage + Serialize + Deserialize<'static>
{
  serde_json::to_string(&msg).unwrap()
}

fn deserialize_to_message<T>(validator: &JSONValidator, msg: String) -> Result<Vec<T>, ButtplugError>
where
  T: serde::de::DeserializeOwned + Clone,
{
  match validator.validate(&msg) {
    Ok(_) => serde_json::from_str::<Vec<T>>(&msg)
      .map_err(|e| ButtplugMessageError::new(&e.to_string()).into()),
    Err(err) => Err(err.into())
  }
}

fn serialize_to_version(
  version: ButtplugMessageSpecVersion,
  msgs: Vec<ButtplugOutMessage>,
) -> ButtplugSerializedMessage {
  ButtplugSerializedMessage::Text(
  match version {
    ButtplugMessageSpecVersion::Version0 => {
      let msg_vec: Vec<ButtplugSpecV0OutMessage> = 
        msgs.iter().cloned().map(|msg| match ButtplugSpecV0OutMessage::try_from(msg) {
          Ok(msgv0) => msgv0,
          Err(err) => ButtplugSpecV0OutMessage::Error(ButtplugError::ButtplugMessageError(err).into())
        }).collect();
      vec_to_protocol_json(msg_vec)
      },
ButtplugMessageSpecVersion::Version1 => {
      let msg_vec: Vec<ButtplugSpecV1OutMessage> = 
        msgs.iter().cloned().map(|msg| match ButtplugSpecV1OutMessage::try_from(msg) {
          Ok(msgv0) => msgv0,
          Err(err) => ButtplugSpecV1OutMessage::Error(ButtplugError::ButtplugMessageError(err).into())
        }).collect();
      vec_to_protocol_json(msg_vec)
      },
      ButtplugMessageSpecVersion::Version2 => {
        let msg_vec: Vec<ButtplugSpecV2OutMessage> = 
          msgs.iter().cloned().map(|msg| match ButtplugSpecV2OutMessage::try_from(msg) {
            Ok(msgv0) => msgv0,
            Err(err) => ButtplugSpecV2OutMessage::Error(ButtplugError::ButtplugMessageError(err).into())
          }).collect();
        vec_to_protocol_json(msg_vec)
        },
  })
}

unsafe impl Sync for ButtplugServerJSONSerializer {}
unsafe impl Send for ButtplugServerJSONSerializer {}

impl ButtplugMessageSerializer for ButtplugServerJSONSerializer {
  type Inbound = ButtplugInMessage;
  type Outbound = ButtplugOutMessage;

  fn deserialize(&mut self, serialized_msg: ButtplugSerializedMessage) -> Result<Vec<ButtplugInMessage>, ButtplugError> {
    let msg  = if let ButtplugSerializedMessage::Text(text_msg) = serialized_msg {
      text_msg
    } else {
      return Err(ButtplugError::ButtplugMessageError(ButtplugMessageError::new("Cannot deserialize binary messages with JSON serializer.")));
    };
    // If we don't have a message version yet, we need to parse this as a
    // RequestServerInfo message to get the version. RequestServerInfo can
    // always be parsed as the latest message version, as we keep it
    // compatible across versions via serde options.
    if let Some(version) = self.message_version {
      Ok(match version {
        ButtplugMessageSpecVersion::Version0 => 
          deserialize_to_message::<ButtplugSpecV0InMessage>(&self.validator, msg)?.iter().cloned().map(|m| m.into()).collect(),
        ButtplugMessageSpecVersion::Version1 => 
          deserialize_to_message::<ButtplugSpecV1InMessage>(&self.validator, msg)?.iter().cloned().map(|m| m.into()).collect(),
        ButtplugMessageSpecVersion::Version2 => 
          deserialize_to_message::<ButtplugSpecV2InMessage>(&self.validator, msg)?.iter().cloned().map(|m| m.into()).collect(),
      })
    } else {
      let msg_union = deserialize_to_message::<ButtplugSpecV2InMessage>(&self.validator, msg)?;
      if let ButtplugSpecV2InMessage::RequestServerInfo(rsi) = &msg_union[0] {
        info!(
          "Setting JSON Wrapper message version to {}",
          rsi.message_version
        );
        self.message_version = Some(rsi.message_version);
      } else {
        return Err(ButtplugError::ButtplugHandshakeError(
          ButtplugHandshakeError::new(
            "First message received must be a RequestServerInfo message.",
          ),
        ));
      }
      Ok(msg_union.iter().cloned().map(|m| m.into()).collect())
    }
  }

  fn serialize(&mut self, msgs: Vec<ButtplugOutMessage>) -> ButtplugSerializedMessage {
    if let Some(version) = self.message_version {
      serialize_to_version(version, msgs)
    } else {
      // In the rare event that there is a problem with the
      // RequestServerInfo message (so we can't set up our known spec
      // version), just encode to the latest and return.
      if let ButtplugOutMessage::Error(_) = &msgs[0] {
        serialize_to_version(
          ButtplugMessageSpecVersion::Version2,
          msgs,
        )
      } else {
        // If we don't even have enough info to know which message
        // version to convert to, consider this a handshake error.
        ButtplugSerializedMessage::Text(msg_to_protocol_json(ButtplugOutMessage::Error(
          ButtplugError::ButtplugHandshakeError(ButtplugHandshakeError::new(
            "Got outgoing message before version was set.",
          ))
          .into(),
        )))
      }
    }
  }
}

pub struct ButtplugClientJSONSerializer {
  validator: JSONValidator
}

impl Default for ButtplugClientJSONSerializer {
  fn default() -> Self {
    Self {
      validator: create_message_validator()
    }
  }
}

unsafe impl Sync for ButtplugClientJSONSerializer {}
unsafe impl Send for ButtplugClientJSONSerializer {}

impl ButtplugMessageSerializer for ButtplugClientJSONSerializer {
  type Inbound = ButtplugClientOutMessage;
  type Outbound = ButtplugClientInMessage;

  fn deserialize(&mut self, msg: ButtplugSerializedMessage) -> Result<Vec<ButtplugClientOutMessage>, ButtplugError> {
    if let ButtplugSerializedMessage::Text(text_msg) = msg {
      deserialize_to_message::<Self::Inbound>(&self.validator, text_msg)
    } else {
      Err(ButtplugError::ButtplugMessageError(ButtplugMessageError::new("Cannot deserialize binary messages with JSON serializer.")))
    }
  }

  fn serialize(&mut self, msg: Vec<ButtplugClientInMessage>) -> ButtplugSerializedMessage {
    ButtplugSerializedMessage::Text(vec_to_protocol_json(msg))
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
                "MessageVersion": 2
            }
        }]"#;
    let mut serializer = ButtplugServerJSONSerializer::default();
    serializer.deserialize(ButtplugSerializedMessage::Text(json.to_owned())).unwrap();
    assert_eq!(serializer.message_version, Some(ButtplugMessageSpecVersion::Version2));
  }

  #[test]
  fn test_wrong_message_version() {
    let json = r#"[{
            "RequestServerInfo": {
                "Id": 1,
                "ClientName": "Test Client",
                "MessageVersion": 100
            }
        }]"#;
    let mut serializer = ButtplugServerJSONSerializer::default();
    let msg = serializer.deserialize(ButtplugSerializedMessage::Text(json.to_owned()));
    assert!(msg.is_err());
  }
}
