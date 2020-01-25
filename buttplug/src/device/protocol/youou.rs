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

pub struct YououProtocolCreator {
    config: DeviceProtocolConfiguration,
}

impl YououProtocolCreator {
    pub fn new(config: DeviceProtocolConfiguration) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ButtplugProtocolCreator for YououProtocolCreator {
    async fn try_create_protocol(
        &self,
        device_impl: &Box<dyn DeviceImpl>,
    ) -> Result<Box<dyn ButtplugProtocol>, ButtplugError> {
        let (names, attrs) = self.config.get_attributes("VX001_").unwrap();
        let name = names.get("en-us").unwrap();
        Ok(Box::new(YououProtocol::new(name, attrs)))
    }
}

#[derive(Clone)]
pub struct YououProtocol {
    name: String,
    attributes: MessageAttributesMap,
    packet_id: u8,
}

impl YououProtocol {
    pub fn new(name: &str, attributes: MessageAttributesMap) -> Self {
        YououProtocol {
            name: name.to_owned(),
            attributes,
            packet_id: 0,
        }
    }
}

#[async_trait]
impl ButtplugProtocol for YououProtocol {
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
                ButtplugDeviceError::new("YououProtocol does not accept this message type."),
            )),
        }
    }
}

impl YououProtocol {
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
        // Byte 2 seems to be a monotonically increasing packet id of some kind Speed seems to be
        // 0-247 or so. Anything above that sets a pattern which isn't what we want here.
        let max_value: f64 = 247.0;
        let speed: u8 = (msg.speeds[0].speed * max_value) as u8;
        let state: u8 = if speed > 0 { 1 } else { 0 };
        let mut data = vec![0xaa, 0x55, self.packet_id, 0x02, 0x03, 0x01, speed, state];
        let mut crc: u8 = 0;

        // Simple XOR of everything up to the 9th byte for CRC.
        for b in data.clone() {
            crc = b ^ crc;
        }

        let mut data2 = vec![crc, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        data.append(&mut data2);

        // Hopefully this will wrap back to 0 at 256
        self.packet_id = self.packet_id.wrapping_add(1);

        let msg = DeviceWriteCmd::new(Endpoint::Tx, data, false);
        device.write_value(msg.into()).await?;
        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    }
}
