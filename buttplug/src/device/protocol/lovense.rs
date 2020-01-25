use super::{ButtplugProtocol, ButtplugProtocolCreator};
use crate::{
    core::{
        errors::{ButtplugDeviceError, ButtplugError},
        messages::{
            self, ButtplugDeviceCommandMessageUnion, ButtplugMessageUnion, MessageAttributesMap,
            RotateCmd, RotationSubcommand, StopDeviceCmd, VibrateCmd, VibrateSubcommand,
        },
    },
    device::{
        configuration_manager::DeviceProtocolConfiguration,
        device::{
            ButtplugDeviceEvent, DeviceImpl, DeviceSubscribeCmd, DeviceUnsubscribeCmd,
            DeviceWriteCmd,
        },
        Endpoint,
    },
};
use async_std::prelude::StreamExt;
use async_trait::async_trait;

pub struct LovenseProtocolCreator {
    config: DeviceProtocolConfiguration,
}

impl LovenseProtocolCreator {
    pub fn new(config: DeviceProtocolConfiguration) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ButtplugProtocolCreator for LovenseProtocolCreator {
    async fn try_create_protocol(
        &self,
        device_impl: &Box<dyn DeviceImpl>,
    ) -> Result<Box<dyn ButtplugProtocol>, ButtplugError> {
        device_impl
            .subscribe(DeviceSubscribeCmd::new(Endpoint::Rx).into())
            .await?;
        let msg = DeviceWriteCmd::new(Endpoint::Tx, "DeviceType;".as_bytes().to_vec(), false);
        device_impl.write_value(msg.into()).await?;
        // TODO Put some sort of very quick timeout here, we should just fail if
        // we don't get something back quickly.
        let identifier;
        match device_impl.get_event_receiver().next().await {
            Some(ButtplugDeviceEvent::Notification(_, n)) => {
                let type_response = std::str::from_utf8(&n).unwrap().to_owned();
                info!("Lovense Device Type Response: {}", type_response);
                identifier = type_response.split(':').collect::<Vec<&str>>()[0].to_owned();
            }
            Some(ButtplugDeviceEvent::Removed) => {
                return Err(ButtplugDeviceError::new(
                    "Lovense Device disconnected while getting DeviceType info.",
                )
                .into());
            }
            None => {
                return Err(ButtplugDeviceError::new(
                    "Did not get DeviceType return from Lovense device in time",
                )
                .into());
            }
        };
        device_impl
            .unsubscribe(DeviceUnsubscribeCmd::new(Endpoint::Rx).into())
            .await?;

        let (names, attrs) = self.config.get_attributes(&identifier).unwrap();
        let name = names.get("en-us").unwrap();
        Ok(Box::new(LovenseProtocol::new(name, attrs)))
    }
}

#[derive(Clone)]
pub struct LovenseProtocol {
    name: String,
    attributes: MessageAttributesMap,
    sent_vibration: bool,
    sent_rotation: bool,
    vibrations: Vec<u32>,
    rotations: Vec<(u32, bool)>,
}

impl LovenseProtocol {
    pub fn new(name: &str, attributes: MessageAttributesMap) -> Self {
        let mut vibrations: Vec<u32> = vec![];
        let mut rotations: Vec<(u32, bool)> = vec![];
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

        LovenseProtocol {
            name: name.to_owned(),
            attributes,
            sent_rotation: false,
            sent_vibration: false,
            vibrations,
            rotations,
        }
    }
}

#[async_trait]
impl ButtplugProtocol for LovenseProtocol {
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
                ButtplugDeviceError::new("LovenseProtocol does not accept this message type."),
            )),
        }
    }
}

impl LovenseProtocol {
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
        if self.rotations.len() > 0 {
            let mut subs: Vec<RotationSubcommand> = vec![];
            for i in 0..self.rotations.len() {
                subs.push(RotationSubcommand::new(i as u32, 0.0, self.rotations[i].1));
            }
            self.handle_rotate_cmd(device, &RotateCmd::new(0, subs))
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
        let mut changed: Vec<bool> = vec![];
        for _ in 0..new_speeds.len() {
            changed.push(!self.sent_vibration);
        }

        if new_speeds.len() == 0 {
            // Should probably be an error
            return Ok(ButtplugMessageUnion::Ok(messages::Ok::default()));
        }

        for i in 0..msg.speeds.len() {
            //ToDo: Need safeguards
            let index = msg.speeds[i].index as usize;
            new_speeds[index] = (msg.speeds[i].speed * 20.0) as u32;
            if new_speeds[index] != self.vibrations[index] {
                changed[index] = true;
            }
        }

        let mut asOne = true;
        if new_speeds.len() > 1 {
            let speed = new_speeds[0];
            for i in 1..new_speeds.len() {
                if new_speeds[i] != speed {
                    asOne = false;
                    break;
                }
            }
        }

        self.sent_vibration = true;
        self.vibrations = new_speeds;

        if asOne {
            if changed[0] {
                let msg = DeviceWriteCmd::new(
                    Endpoint::Tx,
                    format!("Vibrate:{};", self.vibrations[0])
                        .as_bytes()
                        .to_vec(),
                    false,
                );
                device.write_value(msg.into()).await?;
            }
        } else {
            for i in 0..self.vibrations.len() {
                if !changed[i] {
                    continue;
                }

                let msg = DeviceWriteCmd::new(
                    Endpoint::Tx,
                    format!("Vibrate{}:{};", i + 1, self.vibrations[i])
                        .as_bytes()
                        .to_vec(),
                    false,
                );
                device.write_value(msg.into()).await?;
            }
        }

        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    }

    async fn handle_rotate_cmd(
        &mut self,
        device: &Box<dyn DeviceImpl>,
        msg: &RotateCmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        let mut new_rotations = self.rotations.clone();
        let mut messages = vec![];

        if new_rotations.len() == 0 {
            // TODO: Should probably be an error
            return Ok(ButtplugMessageUnion::Ok(messages::Ok::default()));
        }

        let mut changed: Vec<bool> = vec![!self.sent_rotation; new_rotations.len() as usize];

        for i in 0..msg.rotations.len() {
            // TODO: Need safeguards
            let index = msg.rotations[i].index as usize;
            new_rotations[index] = (
                (msg.rotations[i].speed * 20.0) as u32,
                msg.rotations[i].clockwise,
            );
            if new_rotations[index] != self.rotations[index] {
                changed[index] = true;
            }
            if new_rotations[i].1 != self.rotations[i].1 {
                messages.push(DeviceWriteCmd::new(
                    Endpoint::Tx,
                    "RotateChange;".as_bytes().to_vec(),
                    false,
                ));
            }
        }

        self.sent_rotation = true;
        self.rotations = new_rotations;

        if changed[0] {
            messages.push(DeviceWriteCmd::new(
                Endpoint::Tx,
                format!("Rotate:{};", self.rotations[0].0)
                    .as_bytes()
                    .to_vec(),
                false,
            ));
        }

        for msg in messages {
            device.write_value(msg.into()).await?;
        }

        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    }
}
