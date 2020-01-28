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

pub struct LiboSharkProtocolCreator {
    config: DeviceProtocolConfiguration,
}

impl LiboSharkProtocolCreator {
    pub fn new(config: DeviceProtocolConfiguration) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ButtplugProtocolCreator for LiboSharkProtocolCreator {
    async fn try_create_protocol(
        &self,
        device_impl: &Box<dyn DeviceImpl>,
    ) -> Result<Box<dyn ButtplugProtocol>, ButtplugError> {
        let (names, attrs) = self.config.get_attributes(device_impl.name()).unwrap();
        let name = names.get("en-us").unwrap();
        Ok(Box::new(LiboSharkProtocol::new(name, attrs)))
    }
}

#[derive(Clone)]
pub struct LiboSharkProtocol {
    name: String,
    attributes: MessageAttributesMap,
    sent_vibration: bool,
    vibrations: Vec<u8>,
}

impl LiboSharkProtocol {
    pub fn new(name: &str, attributes: MessageAttributesMap) -> Self {
        let mut vibrations: Vec<u8> = vec![];
        if let Some(attr) = attributes.get("VibrateCmd") {
            if let Some(count) = attr.feature_count {
                vibrations = vec![0; count as usize];
            }
        }
        LiboSharkProtocol {
            name: name.to_owned(),
            attributes,
            sent_vibration: false,
            vibrations,
        }
    }
}

#[async_trait]
impl ButtplugProtocol for LiboSharkProtocol {
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
                ButtplugDeviceError::new("LiboSharkProtocol does not accept this message type."),
            )),
        }
    }
}

impl LiboSharkProtocol {
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
        let mut new_speeds = self.vibrations.clone();
        let mut changed: bool = !self.sent_vibration;

        if new_speeds.len() == 0 || new_speeds.len() > 2 {
            // Should probably be an error
            return Ok(ButtplugMessageUnion::Ok(messages::Ok::default()));
        }

        // ToDo: Per-feature step count support?
        let max_values = [3u8, 3];

        for i in 0..msg.speeds.len() {
            //ToDo: Need safeguards
            let index = msg.speeds[i].index as usize;
            new_speeds[index] = (msg.speeds[i].speed * max_values[index] as f64) as u8;
            if new_speeds[index] != self.vibrations[index] {
                changed = true;
            }
        }

        self.sent_vibration = true;
        self.vibrations = new_speeds;

        if changed {
            let mut data = 0u8;
            data |= self.vibrations[0] << 4;
            data |= self.vibrations[1];
            let msg = DeviceWriteCmd::new(Endpoint::Tx, vec![data], false);
            device.write_value(msg.into()).await?;
        }

        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    }
}
