use super::{ButtplugProtocol, ButtplugProtocolCreator};
use crate::core::messages::FleshlightLaunchFW12Cmd;
use crate::util::fleshlight_helper::FleshlightHelper;
use crate::{
    core::{
        errors::{ButtplugDeviceError, ButtplugError},
        messages::{
            self, ButtplugDeviceCommandMessageUnion, ButtplugMessageUnion, LinearCmd,
            MessageAttributesMap, StopDeviceCmd, VibrateCmd, VibrateSubcommand,
        },
    },
    device::{
        configuration_manager::DeviceProtocolConfiguration,
        device::{DeviceImpl, DeviceWriteCmd},
        Endpoint,
    },
};
use async_trait::async_trait;

pub struct KiirooGen21ProtocolCreator {
    config: DeviceProtocolConfiguration,
}

impl KiirooGen21ProtocolCreator {
    pub fn new(config: DeviceProtocolConfiguration) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ButtplugProtocolCreator for KiirooGen21ProtocolCreator {
    async fn try_create_protocol(
        &self,
        device_impl: &Box<dyn DeviceImpl>,
    ) -> Result<Box<dyn ButtplugProtocol>, ButtplugError> {
        let (names, attrs) = self.config.get_attributes(device_impl.name()).unwrap();
        let name = names.get("en-us").unwrap();
        Ok(Box::new(KiirooGen21Protocol::new(name, attrs)))
    }
}

#[derive(Clone)]
pub struct KiirooGen21Protocol {
    name: String,
    attributes: MessageAttributesMap,
    sent_vibration: bool,
    vibrations: Vec<u32>,
    vibration_order: Vec<usize>,
    last_position: f64,
}

impl KiirooGen21Protocol {
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

        KiirooGen21Protocol {
            name: name.to_owned(),
            attributes,
            sent_vibration: false,
            vibrations,
            vibration_order,
            last_position: 0.0,
        }
    }
}

#[async_trait]
impl ButtplugProtocol for KiirooGen21Protocol {
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
            ButtplugDeviceCommandMessageUnion::LinearCmd(msg) => {
                self.handle_linear_cmd(device, msg).await
            }
            ButtplugDeviceCommandMessageUnion::FleshlightLaunchFW12Cmd(msg) => {
                self.handle_fleshlight_lanuch_fw12_cmd(device, msg).await
            }
            _ => Err(ButtplugError::ButtplugDeviceError(
                ButtplugDeviceError::new("KiirooGen21Protocol does not accept this message type."),
            )),
        }
    }
}

impl KiirooGen21Protocol {
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
            [0x01, self.vibrations[self.vibration_order[0]] as u8].to_vec(),
            false,
        );
        device.write_value(msg.into()).await?;

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
        let v = &msg.vectors[0];

        self.last_position = v.position;
        //ToDo: We know the position, the target position and the duration.
        // We can work out if we're being redirected mid move!

        let msg = DeviceWriteCmd::new(
            Endpoint::Tx,
            [
                0x03,
                0x00,
                (FleshlightHelper::get_speed((self.last_position - v.position).abs(), v.duration)
                    * 99.0) as u8,
                (v.position * 99.0) as u8,
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
            [0x03, 0x00, msg.speed, msg.position].to_vec(),
            false,
        );
        device.write_value(msg.into()).await?;

        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    }
}
