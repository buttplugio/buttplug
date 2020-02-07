use super::{ButtplugProtocol, ButtplugProtocolCreator};
use crate::{
    core::{
        errors::{ButtplugDeviceError, ButtplugError},
        messages::{
            self, ButtplugDeviceCommandMessageUnion, ButtplugMessageUnion, MessageAttributesMap,
            StopDeviceCmd, LinearCmd, FleshlightLaunchFW12Cmd,
        },
    },
    device::{
        configuration_manager::DeviceProtocolConfiguration,
        device::{DeviceImpl, DeviceWriteCmd},
        Endpoint,
    },
    util::fleshlight_helper::FleshlightHelper,
};
use async_trait::async_trait;

pub struct KiirooGen2ProtocolCreator {
    config: DeviceProtocolConfiguration,
}

impl KiirooGen2ProtocolCreator {
    pub fn new(config: DeviceProtocolConfiguration) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ButtplugProtocolCreator for KiirooGen2ProtocolCreator {
    async fn try_create_protocol(
        &self,
        device_impl: &Box<dyn DeviceImpl>,
    ) -> Result<Box<dyn ButtplugProtocol>, ButtplugError> {
        let (names, attrs) = self.config.get_attributes(device_impl.name()).unwrap();
        let name = names.get("en-us").unwrap();

        // Initialise
        let msg = DeviceWriteCmd::new(
            Endpoint::Firmware,
            [ 0x00 ]
                .to_vec(),
            false,
        );
        device_impl.write_value(msg.into()).await?;

        Ok(Box::new(KiirooGen2Protocol::new(name, attrs)))
    }
}

#[derive(Clone)]
pub struct KiirooGen2Protocol {
    name: String,
    attributes: MessageAttributesMap,
    last_position: f64,
}

impl KiirooGen2Protocol {
    pub fn new(name: &str, attributes: MessageAttributesMap) -> Self {
        KiirooGen2Protocol {
            name: name.to_owned(),
            attributes,
            last_position: 1.0,
        }
    }
}

#[async_trait]
impl ButtplugProtocol for KiirooGen2Protocol {
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
            ButtplugDeviceCommandMessageUnion::LinearCmd(msg) => {
                self.handle_linear_cmd(device, msg).await
            }
            ButtplugDeviceCommandMessageUnion::FleshlightLaunchFW12Cmd(msg) => {
                self.handle_fleshlight_lanuch_fw12_cmd(device, msg).await
            }
            _ => Err(ButtplugError::ButtplugDeviceError(
                ButtplugDeviceError::new("KiirooGen2Protocol does not accept this message type."),
            )),
        }
    }
}

impl KiirooGen2Protocol {
    async fn handle_stop_device_cmd(
        &mut self,
        device: &Box<dyn DeviceImpl>,
        _: &StopDeviceCmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        // No need/way to stop a linear?
        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    }

    async fn handle_linear_cmd(
        &mut self,
        device: &Box<dyn DeviceImpl>,
        msg: &LinearCmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        if msg.vectors.len() != 1 {
            //ToDo: Should probably be an error
            return Ok(ButtplugMessageUnion::Ok(messages::Ok::default()));
        }
        let v =  &msg.vectors[0];

        //ToDo: We know the position, the target position and the duration.
        // We can work out if we're being redirected mid move!

        let speed = (FleshlightHelper::get_speed((self.last_position - v.position).abs(), v.duration) * 99.0) as u8;
        let position = (v.position * 99.0) as u8;
        debug!("Moving Fleshlight from {} to {} at speed {}", self.last_position, v.position, speed);
        self.last_position = v.position;

        let msg = DeviceWriteCmd::new(
            Endpoint::Tx,
            [
                position,
                speed,
            ]
                .to_vec(),
            false,
        );
        device.write_value(msg.into()).await?;

        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    }

    async fn handle_fleshlight_lanuch_fw12_cmd(
        &mut self,
        device: &Box<dyn DeviceImpl>,
        msg: &FleshlightLaunchFW12Cmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {

        // Repeated logic for removable deprecation
        self.last_position = msg.position as f64 / 99.0;

        let msg = DeviceWriteCmd::new(
            Endpoint::Tx,
            [
                msg.position,
                msg.speed,
            ]
                .to_vec(),
            false,
        );
        device.write_value(msg.into()).await?;

        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    }
}
