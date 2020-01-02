use crate::{
    core::{
        errors::{ButtplugDeviceError, ButtplugError},
        messages::{
            self, ButtplugDeviceCommandMessageUnion, ButtplugMessage, ButtplugMessageUnion,
            RawWriteCmd, RotateCmd, StopDeviceCmd, VibrateCmd,
        },
    },
    devices::{
        protocol::ButtplugProtocol,
        Endpoint,
    },
    server::device_manager::{
        DeviceImpl,
    },
};
use async_trait::async_trait;

#[derive(Clone)]
pub struct LovenseProtocol {}

impl LovenseProtocol {
    pub fn new() -> Self {
        LovenseProtocol { }
    }
}

#[async_trait]
impl ButtplugProtocol for LovenseProtocol {
    async fn initialize(&mut self) {}

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
                self.handle_stop_device_cmd(msg).await
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
        msg: &StopDeviceCmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        Ok(ButtplugMessageUnion::Ok(messages::Ok::new(msg.get_id())))
    }

    async fn handle_vibrate_cmd(
        &self,
        device: &Box<dyn DeviceImpl>,
        msg: &VibrateCmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        let msg = RawWriteCmd::new(
            msg.device_index,
            Endpoint::Tx,
            "Vibrate:20;".as_bytes().to_vec(),
            false,
        );
        device.write_value(&msg).await;
        Ok(ButtplugMessageUnion::Ok(messages::Ok::new(msg.get_id())))
    }

    async fn handle_rotate_cmd(
        &self,
        msg: &RotateCmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        Ok(ButtplugMessageUnion::Ok(messages::Ok::new(msg.get_id())))
    }
}
