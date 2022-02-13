use super::{ButtplugProtocol, ButtplugProtocolCommandHandler, GenericCommandManager};
use crate::{
  core::messages::{ButtplugDeviceCommandMessageUnion, },
  device::{
    protocol::ButtplugProtocolProperties,
    configuration_manager::{ProtocolDeviceAttributes, DeviceAttributesBuilder},
  },
};
use std::sync::Arc;

super::default_protocol_declaration!(RawProtocol);

impl ButtplugProtocolCommandHandler for RawProtocol {
}

// TODO Write tests
