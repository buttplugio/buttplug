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

pub struct LiboElleProtocolCreator {
    config: DeviceProtocolConfiguration,
}

impl LiboElleProtocolCreator {
    pub fn new(config: DeviceProtocolConfiguration) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ButtplugProtocolCreator for LiboElleProtocolCreator {
    async fn try_create_protocol(
        &self,
        device_impl: &Box<dyn DeviceImpl>,
    ) -> Result<Box<dyn ButtplugProtocol>, ButtplugError> {
        let (names, attrs) = self.config.get_attributes(device_impl.name()).unwrap();
        let name = names.get("en-us").unwrap();
        Ok(Box::new(LiboElleProtocol::new(name, attrs)))
    }
}

#[derive(Clone)]
pub struct LiboElleProtocol {
    name: String,
    attributes: MessageAttributesMap,
    sent_vibration: bool,
    vibrations: Vec<u8>,
}

impl LiboElleProtocol {
    pub fn new(name: &str, attributes: MessageAttributesMap) -> Self {
        let mut vibrations: Vec<u8> = vec![];
        if let Some(attr) = attributes.get("VibrateCmd") {
            if let Some(count) = attr.feature_count {
                vibrations = vec![0; count as usize];
            }
        }
        LiboElleProtocol {
            name: name.to_owned(),
            attributes,
            sent_vibration: false,
            vibrations,
        }
    }
}

#[async_trait]
impl ButtplugProtocol for LiboElleProtocol {
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
                ButtplugDeviceError::new("LiboElleProtocol does not accept this message type."),
            )),
        }
    }
}

impl LiboElleProtocol {
    async fn handle_stop_device_cmd(
        &mut self,
        device: &Box<dyn DeviceImpl>,
        _: &StopDeviceCmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        let mut subs: Vec<VibrateSubcommand> = vec![];
        for i in 0..self.vibrations.len() {
            subs.push(VibrateSubcommand::new(i as u32, 0.0));
        }
        self.handle_vibrate_cmd(device, &VibrateCmd::new(0, subs))
            .await
    }

    async fn handle_vibrate_cmd(
        &mut self,
        device: &Box<dyn DeviceImpl>,
        msg: &VibrateCmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        let mut new_speeds = self.vibrations.clone();
        let mut changed: Vec<bool> = vec![];
        for _ in 0..new_speeds.len() {
            changed.push(!self.sent_vibration);
        }

        if new_speeds.len() == 0 || new_speeds.len() > 2 {
            // Should probably be an error
            return Ok(ButtplugMessageUnion::Ok(messages::Ok::default()));
        }

        // ToDo: Per-feature step count support?
        let max_values = [14u8, 3];

        for i in 0..msg.speeds.len() {
            //ToDo: Need safeguards
            let index = msg.speeds[i].index as usize;
            new_speeds[index] = (msg.speeds[i].speed * max_values[index] as f64) as u8;
            if new_speeds[index] != self.vibrations[index] {
                changed[index] = true;
            }
        }

        self.sent_vibration = true;
        self.vibrations = new_speeds;

        if changed[0] {
            let mut data = 0u8;
            if self.vibrations[0] > 0 && self.vibrations[0] <= 7 {
                data |= (self.vibrations[0] - 1) << 4;
                data |= 1; // Set the mode too
            } else if self.vibrations[0] > 7 {
                data |= (self.vibrations[0] - 8) << 4;
                data |= 4; // Set the mode too
            }
            let msg = DeviceWriteCmd::new(Endpoint::Tx, vec![data], false);
            device.write_value(msg.into()).await?;
        }
        if self.vibrations.len() > 1 && changed[1] {
            let msg = DeviceWriteCmd::new(Endpoint::TxMode, vec![self.vibrations[1]], false);
            device.write_value(msg.into()).await?;
        }

        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    }
}
