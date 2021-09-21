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
	let mut command = "".to_string();
	
	for v in msg.vectors() {
		if command.chars().count() > 0 {
			command.push_str(" ");
		}
		
		let position = (v.position * 99f64) as u32;
		let mut index = v.index;
		let mut command_type = 'L';
		if index > 29 {
			index -= 30;
			command_type = 'A';
		}
		else if index > 19 {
			index -= 20;
			command_type = 'V';
		}
		else if index > 9 {
			index -= 10;
			command_type = 'R';
		}
		let command_append = format!("{}{}{:02}I{}", command_type, index, position, v.duration);
		command.push_str(&command_append);
	}
	
	command.push_str("\n");
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