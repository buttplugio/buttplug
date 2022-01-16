use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolCommandHandler};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion, DeviceMessageAttributesMap},
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl,
    DeviceWriteCmd,
    Endpoint,
  },
};
use std::sync::Arc;

super::default_protocol_declaration!(ButtplugPassthru);

impl ButtplugProtocolCommandHandler for ButtplugPassthru {
  fn handle_command(
    &self,
    device: Arc<DeviceImpl>,
    command_message: ButtplugDeviceCommandMessageUnion,
  ) -> ButtplugDeviceResultFuture {
    Box::pin(async move {
      device.write_value(DeviceWriteCmd::new(
        Endpoint::Tx,
        serde_json::to_string(&command_message).expect("Type is always serializable").as_bytes().to_vec(),
        false,
      )).await?;
      Ok(messages::Ok::default().into())
    })
  }
}