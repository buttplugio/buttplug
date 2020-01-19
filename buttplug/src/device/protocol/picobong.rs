use super::{ButtplugProtocol, ButtplugProtocolCreator};
use crate::{
    core::{
        errors::{ButtplugDeviceError, ButtplugError},
        messages::{
            self, ButtplugDeviceCommandMessageUnion, ButtplugMessageUnion, MessageAttributesMap,
            StopDeviceCmd, VibrateCmd, VibrateSubcommand,
        },
    },
    device::{
        configuration_manager::DeviceProtocolConfiguration,
        device::{ DeviceImpl, DeviceWriteCmd },
        Endpoint,
    },
};
use async_trait::async_trait;

pub struct PicobongProtocolCreator {
    config: DeviceProtocolConfiguration,
}

impl PicobongProtocolCreator {
    pub fn new(config: DeviceProtocolConfiguration) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ButtplugProtocolCreator for PicobongProtocolCreator {
    async fn try_create_protocol(
        &self,
        device_impl: &Box<dyn DeviceImpl>,
    ) -> Result<Box<dyn ButtplugProtocol>, ButtplugError> {
        let (names, attrs) = self.config.get_attributes(device_impl.name()).unwrap();
        let name = names.get("en-us").unwrap();
        Ok(Box::new(PicobongProtocol::new(name, attrs)))
    }
}

#[derive(Clone)]
pub struct PicobongProtocol {
    name: String,
    attributes: MessageAttributesMap,
}

impl PicobongProtocol {
    pub fn new(name: &str, attributes: MessageAttributesMap) -> Self {
        PicobongProtocol {
            name: name.to_owned(),
            attributes,
        }
    }
}

#[async_trait]
impl ButtplugProtocol for PicobongProtocol {
    fn name(&self) -> &str {
        &self.name
    }

    fn message_attributes(&self) -> MessageAttributesMap {
        self.attributes.clone()
    }

    fn box_clone(&self) -> Box<dyn ButtplugProtocol> {
        Box::new((*self).clone())
    }

    async fn parse_message(
        &mut self,
        device: &Box<dyn DeviceImpl>,
        message: &ButtplugDeviceCommandMessageUnion,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        match message {
            ButtplugDeviceCommandMessageUnion::StopDeviceCmd(msg) => {
                self.handle_stop_device_cmd(device, msg).await
            }
            ButtplugDeviceCommandMessageUnion::VibrateCmd(msg) => {
                self.handle_vibrate_cmd(device, msg).await
            }
            _ => Err(ButtplugError::ButtplugDeviceError(
                ButtplugDeviceError::new("PicobongProtocol does not accept this message type."),
            )),
        }
    }
}

impl PicobongProtocol {
    async fn handle_stop_device_cmd(
        &self,
        device: &Box<dyn DeviceImpl>,
        _: &StopDeviceCmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        self.handle_vibrate_cmd(
            device,
            &VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.0)]),
        )
        .await
    }

    async fn handle_vibrate_cmd(
        &self,
        device: &Box<dyn DeviceImpl>,
        msg: &VibrateCmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        let speed = (msg.speeds[0].speed * 10.0) as u8;
        let mode: u8 = if speed == 0 { 0xff } else { 0x01 };
        let msg = DeviceWriteCmd::new(
            Endpoint::Tx,
            [ 0x01, mode, speed ].to_vec(),
            false,
        );
        device.write_value(msg.into()).await?;
        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    }
}
