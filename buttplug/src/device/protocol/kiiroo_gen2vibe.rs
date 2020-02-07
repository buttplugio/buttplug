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

pub struct KiirooGen2VibeProtocolCreator {
    config: DeviceProtocolConfiguration,
}

impl KiirooGen2VibeProtocolCreator {
    pub fn new(config: DeviceProtocolConfiguration) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ButtplugProtocolCreator for KiirooGen2VibeProtocolCreator {
    async fn try_create_protocol(
        &self,
        device_impl: &Box<dyn DeviceImpl>,
    ) -> Result<Box<dyn ButtplugProtocol>, ButtplugError> {
        let (names, attrs) = self.config.get_attributes(device_impl.name()).unwrap();
        let name = names.get("en-us").unwrap();
        Ok(Box::new(KiirooGen2VibeProtocol::new(name, attrs)))
    }
}

#[derive(Clone)]
pub struct KiirooGen2VibeProtocol {
    name: String,
    attributes: MessageAttributesMap,
    sent_vibration: bool,
    vibrations: Vec<u32>,
    vibration_order: Vec<usize>,
}

impl KiirooGen2VibeProtocol {
    pub fn new(name: &str, attributes: MessageAttributesMap) -> Self {
        let mut vibrations: Vec<u32> = vec![];
        let mut vibration_order: Vec<usize> = vec![];
        if let Some(attr) = attributes.get("VibrateCmd") {
            if let Some(count) = attr.feature_count {
                vibrations = vec![0; count as usize];
            }
            if !vibrations.is_empty() {
                if let Some(order) = &attr.feature_order {
                    for i in 0..vibrations.len() {
                        if order.len() > i
                            && !vibration_order.contains(&(order[i] as usize))
                            && (order[i] as usize) < vibration_order.len()
                        {
                            vibration_order.push(i);
                        } else {
                            warn!(
                                "Feature order '{:?}' not valid for a count of {}",
                                order,
                                vibrations.len()
                            );
                            vibration_order = vec![];
                        }
                    }
                }

                // Handle no order or bad order
                if vibration_order.len() != vibrations.len() {
                    for i in 0..vibrations.len() {
                        vibration_order.push(i);
                    }
                }
            }
        }

        KiirooGen2VibeProtocol {
            name: name.to_owned(),
            attributes,
            sent_vibration: false,
            vibrations,
            vibration_order,
        }
    }
}

#[async_trait]
impl ButtplugProtocol for KiirooGen2VibeProtocol {
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
                ButtplugDeviceError::new(
                    "KiirooGen2VibeProtocol does not accept this message type.",
                ),
            )),
        }
    }
}

impl KiirooGen2VibeProtocol {
    async fn handle_stop_device_cmd(
        &mut self,
        device: &Box<dyn DeviceImpl>,
        _: &StopDeviceCmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        if self.vibrations.len() > 0 {
            let mut subs: Vec<VibrateSubcommand> = vec![];
            for i in 0..self.vibrations.len() {
                subs.push(VibrateSubcommand::new(i as u32, 0.0));
            }
            self.handle_vibrate_cmd(device, &VibrateCmd::new(0, subs))
                .await;
        }
        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    }

    async fn handle_vibrate_cmd(
        &mut self,
        device: &Box<dyn DeviceImpl>,
        msg: &VibrateCmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        let mut new_speeds = self.vibrations.clone();
        let mut changed: bool = !self.sent_vibration;

        if new_speeds.len() == 0 {
            // Should probably be an error
            return Ok(ButtplugMessageUnion::Ok(messages::Ok::default()));
        }

        for i in 0..msg.speeds.len() {
            //ToDo: Need safeguards
            let index = msg.speeds[i].index as usize;
            new_speeds[index] = (msg.speeds[i].speed * 100.0) as u32;
            if new_speeds[index] != self.vibrations[index] {
                changed = true;
            }
        }

        self.sent_vibration = true;
        self.vibrations = new_speeds;

        if !changed {
            return Ok(ButtplugMessageUnion::Ok(messages::Ok::default()));
        }

        let msg = DeviceWriteCmd::new(
            Endpoint::Tx,
            [
                self.vibrations[self.vibration_order[0]] as u8,
                if self.vibrations.len() >= 2 {self.vibrations[self.vibration_order[1]] as u8} else {0},
                if self.vibrations.len() >= 3 {self.vibrations[self.vibration_order[2]] as u8} else {0},
            ]
            .to_vec(),
            false,
        );
        device.write_value(msg.into()).await?;

        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    }
}
