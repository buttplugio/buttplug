use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolCommandHandler};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion, DeviceMessageAttributesMap},
  device::{
    protocol::ButtplugProtocolProperties,
    DeviceImpl, DeviceWriteCmd, Endpoint,
  },
};
use std::sync::Arc;

#[derive(ButtplugProtocolProperties)]
pub struct TCodeV03 {
  name: String,
  message_attributes: DeviceMessageAttributesMap,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl ButtplugProtocol for TCodeV03 {
  fn new_protocol(
    name: &str,
    message_attributes: DeviceMessageAttributesMap,
  ) -> Box<dyn ButtplugProtocol>
  where
    Self: Sized,
  {
    Box::new(Self {
      name: name.to_owned(),
      message_attributes,
      stop_commands: vec!(),
    })
  }
}

impl ButtplugProtocolCommandHandler for TCodeV03 {
  fn handle_linear_cmd(
    &self,
    device: Arc<DeviceImpl>,
    msg: messages::LinearCmd,
  ) -> ButtplugDeviceResultFuture {
    let v = msg.vectors()[0].clone();

    let position = (v.position * 99f64) as u32;

    let command = format!("L0{:02}I{}\n", position, v.duration);
    info!("{}", command);
    let fut = device.write_value(DeviceWriteCmd::new(
      Endpoint::Tx,
      command.as_bytes().to_vec(),
      false,
    ));

    Box::pin(async move {
      fut.await?;
      Ok(messages::Ok::default().into())
    })
  }
}