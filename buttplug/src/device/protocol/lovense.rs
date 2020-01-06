use super::{ButtplugProtocol, ButtplugProtocolCreator};
use crate::{
    core::{
        errors::{ButtplugDeviceError, ButtplugError},
        messages::{
            self, ButtplugDeviceCommandMessageUnion, ButtplugMessageUnion, MessageAttributesMap,
            RotateCmd, StopDeviceCmd, VibrateCmd, VibrateSubcommand,
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
}

impl LovenseProtocol {
    pub fn new(name: &str, attributes: MessageAttributesMap) -> Self {
        LovenseProtocol {
            name: name.to_owned(),
            attributes,
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
            ButtplugDeviceCommandMessageUnion::RotateCmd(msg) => self.handle_rotate_cmd(msg).await,
            _ => Err(ButtplugError::ButtplugDeviceError(
                ButtplugDeviceError::new("LovenseProtocol does not accept this message type."),
            )),
        }
    }
}

impl LovenseProtocol {
    async fn handle_stop_device_cmd(
        &self,
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
        &self,
        device: &Box<dyn DeviceImpl>,
        msg: &VibrateCmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        let msg = DeviceWriteCmd::new(
            Endpoint::Tx,
            format!("Vibrate:{};", (msg.speeds[0].speed * 20.0) as u32)
                .as_bytes()
                .to_vec(),
            false,
        );
        device.write_value(msg.into()).await?;
        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    }

    async fn handle_rotate_cmd(
        &self,
        _msg: &RotateCmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    }
}
