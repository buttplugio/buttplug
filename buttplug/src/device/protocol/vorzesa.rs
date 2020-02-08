use super::{ButtplugProtocol, ButtplugProtocolCreator};
use crate::core::messages::RotateCmd;
use crate::{
    core::{
        errors::{ButtplugDeviceError, ButtplugError},
        messages::{
            self, ButtplugDeviceCommandMessageUnion, ButtplugMessageUnion, MessageAttributesMap,
            RotationSubcommand, StopDeviceCmd, VibrateCmd, VibrateSubcommand,
        },
    },
    device::{
        configuration_manager::DeviceProtocolConfiguration,
        device::{DeviceImpl, DeviceWriteCmd},
        Endpoint,
    },
};
use async_trait::async_trait;

pub struct VorzeSAProtocolCreator {
    config: DeviceProtocolConfiguration,
}

impl VorzeSAProtocolCreator {
    pub fn new(config: DeviceProtocolConfiguration) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ButtplugProtocolCreator for VorzeSAProtocolCreator {
    async fn try_create_protocol(
        &self,
        device_impl: &Box<dyn DeviceImpl>,
    ) -> Result<Box<dyn ButtplugProtocol>, ButtplugError> {
        let (names, attrs) = self.config.get_attributes(device_impl.name()).unwrap();
        let name = names.get("en-us").unwrap();
        Ok(Box::new(VorzeSAProtocol::new(name, attrs)))
    }
}

#[derive(Clone)]
pub struct VorzeSAProtocol {
    name: String,
    attributes: MessageAttributesMap,
    sent_vibration: bool,
    sent_rotation: bool,
    vibrations: Vec<u8>,
    rotations: Vec<(u8, bool)>,
}

impl VorzeSAProtocol {
    pub fn new(name: &str, attributes: MessageAttributesMap) -> Self {
        let mut vibrations: Vec<u8> = vec![];
        let mut rotations: Vec<(u8, bool)> = vec![];
        if let Some(attr) = attributes.get("VibrateCmd") {
            if let Some(count) = attr.feature_count {
                vibrations = vec![0; count as usize];
            }
        }
        if let Some(attr) = attributes.get("RotateCmd") {
            if let Some(count) = attr.feature_count {
                rotations = vec![(0, true); count as usize];
            }
        }

        VorzeSAProtocol {
            name: name.to_owned(),
            attributes,
            sent_vibration: false,
            sent_rotation: false,
            vibrations,
            rotations,
        }
    }
}

#[async_trait]
impl ButtplugProtocol for VorzeSAProtocol {
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
            ButtplugDeviceCommandMessageUnion::RotateCmd(msg) => {
                self.handle_rotate_cmd(device, msg).await
            }
            _ => Err(ButtplugError::ButtplugDeviceError(
                ButtplugDeviceError::new("VorzeSAProtocol does not accept this message type."),
            )),
        }
    }
}

impl VorzeSAProtocol {
    async fn handle_stop_device_cmd(
        &mut self,
        device: &Box<dyn DeviceImpl>,
        _: &StopDeviceCmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        if !self.vibrations.is_empty() {
            self.handle_vibrate_cmd(
                device,
                &VibrateCmd::new(
                    0,
                    vec![VibrateSubcommand::new(0, 0.0); self.vibrations.len()],
                ),
            )
            .await?;
        }
        if !self.rotations.is_empty() {
            self.handle_rotate_cmd(
                device,
                &RotateCmd::new(
                    0,
                    vec![RotationSubcommand::new(0, 0.0, true); self.rotations.len()],
                ),
            )
            .await?;
        }
        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
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
        let max_value: u8 = 100;

        for i in 0..msg.speeds.len() {
            //ToDo: Need safeguards
            let index = msg.speeds[i].index as usize;
            new_speeds[index] = (msg.speeds[i].speed * max_value as f64) as u8;
            if new_speeds[index] != self.vibrations[index] {
                changed[index] = true;
            }
        }

        self.sent_vibration = true;
        self.vibrations = new_speeds;

        if !changed.contains(&true) {
            return Ok(ButtplugMessageUnion::Ok(messages::Ok::default()));
        }

        // 6 = bach
        // 3 = vibrate

        let msg = DeviceWriteCmd::new(Endpoint::Tx, vec![0x06, 0x03, self.vibrations[0]], false);
        device.write_value(msg.into()).await?;

        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    }

    async fn handle_rotate_cmd(
        &mut self,
        device: &Box<dyn DeviceImpl>,
        msg: &RotateCmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        let mut new_speeds = self.rotations.clone();
        let mut changed: Vec<bool> = vec![];
        for _ in 0..new_speeds.len() {
            changed.push(!self.sent_rotation);
        }

        if new_speeds.len() == 0 || new_speeds.len() > 2 {
            // Should probably be an error
            return Ok(ButtplugMessageUnion::Ok(messages::Ok::default()));
        }

        // ToDo: Per-feature step count support?
        let max_value: u8 = 100;

        for i in 0..msg.rotations.len() {
            //ToDo: Need safeguards
            let index = msg.rotations[i].index as usize;
            new_speeds[index].0 = (msg.rotations[i].speed * max_value as f64) as u8;
            new_speeds[index].1 = msg.rotations[i].clockwise;
            if new_speeds[index].0 != self.rotations[index].0
                || new_speeds[index].1 != self.rotations[index].1
            {
                changed[index] = true;
            }
        }

        self.sent_vibration = true;
        self.rotations = new_speeds;

        if !changed.contains(&true) {
            return Ok(ButtplugMessageUnion::Ok(messages::Ok::default()));
        }

        // 1 = cylone
        // 2 = ufo
        let dev_id = if self.name.contains("UFO") {
            0x02
        } else {
            0x01
        };

        // 1 = vibrate

        let mut data = (if self.rotations[0].1 { 1 } else { 0 }) << 7;
        data |= self.rotations[0].0;

        let msg = DeviceWriteCmd::new(Endpoint::Tx, vec![dev_id, 0x01, data], false);
        device.write_value(msg.into()).await?;

        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    }
}
