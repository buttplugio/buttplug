use crate::{
    core::{
        errors::{ButtplugDeviceError, ButtplugError},
        messages::{
            self, ButtplugDeviceCommandMessageUnion, ButtplugMessage, ButtplugMessageUnion,
            RawWriteCmd, RotateCmd, StopDeviceCmd, VibrateCmd, VibrateSubcommand, SubscribeCmd,
            UnsubscribeCmd
        },
    },
    device::{
        protocol::ButtplugProtocol,
        Endpoint,
        device::{DeviceImpl, ButtplugDeviceEvent, DeviceSubscribeCmd, DeviceUnsubscribeCmd, DeviceWriteCmd},
    },
};
use async_trait::async_trait;
use async_std::prelude::StreamExt;

#[derive(Clone)]
pub struct LovenseProtocol {}

impl LovenseProtocol {
    pub fn new() -> Self {
        LovenseProtocol { }
    }
}

#[async_trait]
impl ButtplugProtocol for LovenseProtocol {
    async fn initialize(&mut self,
                        device: &Box<dyn DeviceImpl>) {
        device.subscribe(DeviceSubscribeCmd::new(Endpoint::Rx).into()).await;
        let msg = DeviceWriteCmd::new(
            Endpoint::Tx,
            "DeviceType;".as_bytes().to_vec(),
            false,
        );
        device.write_value(msg.into()).await;
        if let Some(ButtplugDeviceEvent::Notification(_, n)) = device.get_event_receiver().next().await {
            info!("{}", std::str::from_utf8(&n).unwrap());
        }
        device.unsubscribe(DeviceUnsubscribeCmd::new(Endpoint::Rx).into()).await;
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
        self.handle_vibrate_cmd(device, &VibrateCmd::new(0, vec!(VibrateSubcommand::new(0, 0.0)))).await
    }

    async fn handle_vibrate_cmd(
        &self,
        device: &Box<dyn DeviceImpl>,
        msg: &VibrateCmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        let msg = DeviceWriteCmd::new(
            Endpoint::Tx,
            format!("Vibrate:{};", (msg.speeds[0].speed * 20.0) as u32).as_bytes().to_vec(),
            false,
        );
        device.write_value(msg.into()).await;
        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    }

    async fn handle_rotate_cmd(
        &self,
        msg: &RotateCmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    }
}
