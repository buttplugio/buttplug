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
        device::{DeviceImpl, DeviceWriteCmd},
        Endpoint,
    },
};
use async_trait::async_trait;

pub struct MaxproProtocolCreator {
    config: DeviceProtocolConfiguration,
}

impl MaxproProtocolCreator {
    pub fn new(config: DeviceProtocolConfiguration) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ButtplugProtocolCreator for MaxproProtocolCreator {
    async fn try_create_protocol(
        &self,
        device_impl: &Box<dyn DeviceImpl>,
    ) -> Result<Box<dyn ButtplugProtocol>, ButtplugError> {
        let (names, attrs) = self.config.get_attributes(device_impl.name()).unwrap();
        let name = names.get("en-us").unwrap();
        Ok(Box::new(MaxproProtocol::new(name, attrs)))
    }
}

#[derive(Clone)]
pub struct MaxproProtocol {
    name: String,
    attributes: MessageAttributesMap,
}

impl MaxproProtocol {
    pub fn new(name: &str, attributes: MessageAttributesMap) -> Self {
        MaxproProtocol {
            name: name.to_owned(),
            attributes,
        }
    }
}

#[async_trait]
impl ButtplugProtocol for MaxproProtocol {
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
                ButtplugDeviceError::new("MaxproProtocol does not accept this message type."),
            )),
        }
    }
}

impl MaxproProtocol {
    async fn handle_stop_device_cmd(
        &mut self,
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
        &mut self,
        device: &Box<dyn DeviceImpl>,
        msg: &VibrateCmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        // Speed range for Maxpro toys are 10-100 for some reason.
        let max_value: f64 = 100.0;
        let speed: u8 = (msg.speeds[0].speed * max_value) as u8;
        let mut data = vec![0x55, 0x04, 0x07, 0xff, 0xff, 0x3f, speed, 0x5f, speed, 0x00];
        let mut crc: u8 = 0;

        for b in data.clone() {
            crc = crc.wrapping_add(b);
        }

        data[9] = crc;

        let msg = DeviceWriteCmd::new(Endpoint::Tx, data, false);
        device.write_value(msg.into()).await?;
        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    }
}
