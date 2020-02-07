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

pub struct WeVibe8bitProtocolCreator {
    config: DeviceProtocolConfiguration,
}

impl WeVibe8bitProtocolCreator {
    pub fn new(config: DeviceProtocolConfiguration) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ButtplugProtocolCreator for WeVibe8bitProtocolCreator {
    async fn try_create_protocol(
        &self,
        device_impl: &Box<dyn DeviceImpl>,
    ) -> Result<Box<dyn ButtplugProtocol>, ButtplugError> {
        let (names, attrs) = self.config.get_attributes(device_impl.name()).unwrap();
        let name = names.get("en-us").unwrap();
        Ok(Box::new(WeVibe8bitProtocol::new(name, attrs)))
    }
}

#[derive(Clone)]
pub struct WeVibe8bitProtocol {
    name: String,
    attributes: MessageAttributesMap,
    sent_vibration: bool,
    vibrations: Vec<u8>,
    max_vibration: u8,
}

impl WeVibe8bitProtocol {
    pub fn new(name: &str, attributes: MessageAttributesMap) -> Self {
        let mut vibrations: Vec<u8> = vec![];
        let mut max_vibration: u8 = 12;
        if let Some(attr) = attributes.get("VibrateCmd") {
            if let Some(count) = attr.feature_count {
                vibrations = vec![0; count as usize];
            }
            if let Some(steps) = &attr.step_count {
                if !steps.is_empty() {
                    max_vibration = steps[0] as u8;
                }
            }
        }
       WeVibe8bitProtocol {
            name: name.to_owned(),
            attributes,
            sent_vibration: false,
            vibrations,
            max_vibration,
        }
    }
}

#[async_trait]
impl ButtplugProtocol for WeVibe8bitProtocol {
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
                ButtplugDeviceError::new("AnerosProtocol does not accept this message type."),
            )),
        }
    }
}

impl WeVibe8bitProtocol {
    async fn handle_stop_device_cmd(
        &mut self,
        device: &Box<dyn DeviceImpl>,
        _: &StopDeviceCmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        self.handle_vibrate_cmd(
            device,
            &VibrateCmd::new(
                0,
                vec![VibrateSubcommand::new(0, 0.0); self.vibrations.len()],
            ),
        )
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
        let max_value: u8 = self.max_vibration;

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

        let mut data = vec![ 0x0f, 0x03, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00 ];
        let speed_int = self.vibrations[0];
        let speed_ext = self.vibrations[self.vibrations.len() - 1];

        data[3] = self.vibrations[self.vibrations.len() - 1] + 3; // External
        data[4] = self.vibrations[0] + 3; // Internal
        data[5] = if self.vibrations[0] == 0 {0} else {1};
        data[5] |= if self.vibrations[self.vibrations.len() - 1] == 0 {0} else {2};

        if self.vibrations[0] == 0 && self.vibrations[self.vibrations.len() - 1] == 0 {
            data[1] = 0x00;
            data[3] = 0x00;
            data[4] = 0x00;
            data[5] = 0x00;
        }

        let msg = DeviceWriteCmd::new(Endpoint::Tx, data, false);
        device.write_value(msg.into()).await?;

        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    }
}
