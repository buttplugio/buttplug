use super::{ButtplugProtocol, ButtplugProtocolCommandHandler, ButtplugProtocolCreator};
use crate::{
  core::{
    messages::{ButtplugDeviceCommandMessageUnion, MessageAttributesMap},
  },
  device::{
    protocol::ButtplugProtocolProperties,
  },
};

#[derive(ButtplugProtocol, ButtplugProtocolCreator, ButtplugProtocolProperties)]
pub struct RawProtocol {
  name: String,
  message_attributes: MessageAttributesMap,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl RawProtocol {
  pub(super) fn new(name: &str, message_attributes: MessageAttributesMap) -> Self {
    Self {
      name: name.to_owned(),
      message_attributes,
      stop_commands: vec!(),
    }
  }
}

impl ButtplugProtocolCommandHandler for RawProtocol {
}

// TODO Write tests
