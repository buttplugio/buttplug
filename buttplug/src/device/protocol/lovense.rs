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
    vibration_speeds: Vec<u32>,
    rotation_speeds: Vec<u32>,
    rotation_clockwise: Vec<bool>,
}

impl LovenseProtocol {
    pub fn new(name: &str, attributes: MessageAttributesMap) -> Self {
        let mut vSpeeds: Vec<u32> = vec![];
        let mut rSpeeds: Vec<u32> = vec![];
        let mut rClockwise: Vec<bool> = vec![];
        if let Some( vAttr) = attributes.get("VibrateCmd") {
            if let Some(count) = vAttr.feature_count {
                for _ in 0..count {
                    vSpeeds.push(0);
                }
            }
        }
        if let Some( vAttr) = attributes.get("RotateCmd") {
            if let Some(count) = vAttr.feature_count {
                for _ in 0..count {
                    rSpeeds.push(0);
                    rClockwise.push(true);
                }
            }
        }

        LovenseProtocol {
            name: name.to_owned(),
            attributes,
            sent_rotation: false,
            sent_vibration: false,
            vibration_speeds: vSpeeds,
            rotation_speeds: rSpeeds,
            rotation_clockwise: rClockwise,
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
        if self.vibration_speeds.len() > 0 {
            let mut subs: Vec<VibrateSubcommand> = vec![];
            for i in 0..self.vibration_speeds.len() {
                subs.push(VibrateSubcommand::new(i as u32, 0.0));
            }
            self.handle_vibrate_cmd(device, &VibrateCmd::new(0, subs)).await;
        }
        if self.rotation_speeds.len() > 0 {
            let mut subs: Vec<RotationSubcommand> = vec![];
            for i in 0..self.rotation_speeds.len() {
                subs.push(RotationSubcommand::new(i as u32, 0.0, self.rotation_clockwise[i]));
            }
            self.handle_rotate_cmd(device, &RotateCmd::new(0, subs)).await;
        }
        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    }

    async fn handle_vibrate_cmd(
        &mut self,
        device: &Box<dyn DeviceImpl>,
        msg: &VibrateCmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        let mut newSpeeds = self.vibration_speeds.clone();
        let mut changed: Vec<bool> = vec![];
        for _ in 0..newSpeeds.len() {
            changed.push(!self.sent_vibration);
        }

        if newSpeeds.len() == 0 {
            // Should probably be an error
            return Ok(ButtplugMessageUnion::Ok(messages::Ok::default()));
        }

        for i in 0..msg.speeds.len() {
            //ToDo: Need safeguards
            let index = msg.speeds[i].index as usize;
            newSpeeds[index] = (msg.speeds[i].speed * 20.0) as u32;
            if newSpeeds[index] != self.vibration_speeds[index] {
                changed[index] = true;
            }
        }

        let mut asOne = true;
        if newSpeeds.len() > 1 {
            let speed = newSpeeds[0];
            for i in 1..newSpeeds.len() {
                if newSpeeds[i] != speed {
                    asOne = false;
                    break;
                }
            }
        }

        self.sent_vibration = true;
        self.vibration_speeds = newSpeeds;

        if asOne {
            if changed[0] {
                let msg = DeviceWriteCmd::new(
                    Endpoint::Tx,
                    format!("Vibrate:{};", self.vibration_speeds[0])
                        .as_bytes()
                        .to_vec(),
                    false,
                );
                device.write_value(msg.into()).await?;
            }
        } else {
            for i in 0..self.vibration_speeds.len() {
                if !changed[i] {
                    continue;
                }

                let msg = DeviceWriteCmd::new(
                    Endpoint::Tx,
                    format!("Vibrate{}:{};", i+1, self.vibration_speeds[i])
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
        let mut newSpeeds = self.rotation_speeds.clone();
        let mut newClockwise = self.rotation_clockwise.clone();
        let mut changed: Vec<bool> = vec![];
        for _ in 0..newSpeeds.len() {
            changed.push(!self.sent_rotation);
        }

        if newSpeeds.len() == 0 {
            // Should probably be an error
            return Ok(ButtplugMessageUnion::Ok(messages::Ok::default()));
        }

        for i in 0..msg.rotations.len() {
            //ToDo: Need safeguards
            let index = msg.rotations[i].index as usize;
            newSpeeds[index] = (msg.rotations[i].speed * 20.0) as u32;
            newClockwise[index] = msg.rotations[i].clockwise;
            if newSpeeds[index] != self.rotation_speeds[index] {
                changed[index] = true;
            }
        }

        for i in 0..newClockwise.len() {
            if newClockwise[i] != self.rotation_clockwise[i] {
                let msg = DeviceWriteCmd::new(
                    Endpoint::Tx,
                    "RotateChange;"
                        .as_bytes()
                        .to_vec(),
                    false,
                );
                device.write_value(msg.into()).await?;
            }
        }

        self.sent_rotation = true;
        self.rotation_speeds = newSpeeds;
        self.rotation_clockwise = newClockwise;

        if changed[0] {
            let msg = DeviceWriteCmd::new(
                Endpoint::Tx,
                format!("Rotate:{};", self.rotation_speeds[0])
                    .as_bytes()
                    .to_vec(),
                false,
            );
            device.write_value(msg.into()).await?;
        }

        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    }
}
