use super::{ButtplugProtocol, ButtplugProtocolCommandHandler, GenericCommandManager};
use crate::{
  core::messages::{ButtplugDeviceCommandMessageUnion, DeviceMessageAttributesMap},
  device::protocol::ButtplugProtocolProperties,
};
use std::sync::Arc;

super::default_protocol_declaration!(RawProtocol);

impl ButtplugProtocolCommandHandler for RawProtocol {
}

// TODO Write tests
