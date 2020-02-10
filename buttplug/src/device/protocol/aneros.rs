use super::{ButtplugProtocol, ButtplugProtocolCreator, GenericCommandManager};
use crate::{
    create_buttplug_protocol_impl,
    core::{
        errors::{ButtplugDeviceError, ButtplugError},
        messages::{
            self, ButtplugDeviceCommandMessageUnion, ButtplugMessageUnion, MessageAttributesMap,
            StopDeviceCmd, VibrateCmd,
        },
    },
    device::{
        configuration_manager::DeviceProtocolConfiguration,
        device::{DeviceImpl, DeviceWriteCmd},
        Endpoint,
    },
};
use async_trait::async_trait;
use async_std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AnerosProtocol {
    name: String,
    attributes: MessageAttributesMap,
    manager: Arc<Mutex<GenericCommandManager>>,
}

impl AnerosProtocol {
    pub fn new(name: &str, attributes: MessageAttributesMap) -> Self {
        AnerosProtocol {
            name: name.to_owned(),
            // Borrow attributes before we store it.
            manager: Arc::new(Mutex::new(GenericCommandManager::new(&attributes))),
            attributes,
        }
    }
}

create_buttplug_protocol_impl!(AnerosProtocol,
    (VibrateCmd, handle_vibrate_cmd),
    (StopDeviceCmd, handle_stop_device_cmd)
);

impl AnerosProtocol {
    async fn handle_stop_device_cmd(
        &mut self,
        device: &Box<dyn DeviceImpl>,
        _: &StopDeviceCmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        let msg = &self.manager.lock().await.create_vibration_stop_cmd();
        self.handle_vibrate_cmd(
            device,
            msg,
        )
        .await
    }

    async fn handle_vibrate_cmd(
        &mut self,
        device: &Box<dyn DeviceImpl>,
        msg: &VibrateCmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        // Store off result before the match, so we drop the lock ASAP.
        let result = self.manager.lock().await.update_vibration(msg);
        // My life for an async closure so I could just do this via and_then(). :(
        match result {
            Ok(cmds) => {
                let mut index = 0u8;
                for cmd in cmds {
                    if let Some(speed) = cmd {
                        device.write_value(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF1 + index, speed as u8], false)).await?;
                    }
                    index += 1;
                }
                Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
            },
            Err(e) => Err(e)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        core::messages::{VibrateCmd, VibrateSubcommand, StopDeviceCmd},
        test::test_device::{TestDevice},
        device::{
            Endpoint,
            device::{DeviceImplCommand, DeviceWriteCmd},
        }
    };
    use async_std::{
        task,
        sync::Receiver,
    };

    pub async fn check_recv_value(receiver: &Receiver<DeviceImplCommand>, command: DeviceImplCommand) {
        assert!(!receiver.is_empty());
        assert_eq!(receiver.recv().await.unwrap(), command);
    }

    #[test]
    pub fn test_aneros_protocol() {
        task::block_on(async move {
            let (mut device, test_device) = TestDevice::new_bluetoothle_test_device("Massage Demo").await.unwrap();
            device.parse_message(&VibrateCmd::new(0, vec!(VibrateSubcommand::new(0, 0.5))).into()).await.unwrap();
            let (_, command_receiver) = test_device.get_endpoint_channel_clone(&Endpoint::Tx).await;
            check_recv_value(&command_receiver, DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF1, 63], false))).await;
            // Since we only created one subcommand, we should only receive one command.
            device.parse_message(&VibrateCmd::new(0, vec!(VibrateSubcommand::new(0, 0.5))).into()).await.unwrap();
            assert!(command_receiver.is_empty());
            device.parse_message(&VibrateCmd::new(0, vec!(VibrateSubcommand::new(0, 0.1), VibrateSubcommand::new(1, 0.5))).into()).await.unwrap();
            // TODO There's probably a more concise way to do this.
            check_recv_value(&command_receiver, DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF1, 12], false))).await;
            check_recv_value(&command_receiver, DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF2, 63], false))).await;
            device.parse_message(&StopDeviceCmd::new(0).into()).await.unwrap();
            check_recv_value(&command_receiver, DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF1, 0], false))).await;
            check_recv_value(&command_receiver, DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF2, 0], false))).await;
        });
    }
}
