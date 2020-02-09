use super::{ButtplugProtocol, ButtplugProtocolCreator};
use crate::{
    create_buttplug_protocol_impl,
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

#[derive(Clone)]
pub struct AnerosProtocol {
    name: String,
    attributes: MessageAttributesMap,
    sent_vibration: bool,
    vibrations: Vec<u8>,
}

impl AnerosProtocol {
    pub fn new(name: &str, attributes: MessageAttributesMap) -> Self {
        let mut vibrations: Vec<u8> = vec![];
        if let Some(attr) = attributes.get("VibrateCmd") {
            if let Some(count) = attr.feature_count {
                vibrations = vec![0; count as usize];
            }
        }
        AnerosProtocol {
            name: name.to_owned(),
            attributes,
            sent_vibration: false,
            vibrations,
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
        let max_value: u8 = 0x7F;

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

        if changed[0] {
            let msg = DeviceWriteCmd::new(Endpoint::Tx, vec![0xF1, self.vibrations[0]], false);
            device.write_value(msg.into()).await?;
        }

        if changed[1] {
            let msg = DeviceWriteCmd::new(Endpoint::Tx, vec![0xF2, self.vibrations[1]], false);
            device.write_value(msg.into()).await?;
        }

        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    }
}

#[cfg(test)]
mod test {
    use crate::{
        core::messages::{VibrateCmd, VibrateSubcommand},
        test::test_device::{TestDevice, TestDeviceImplCreator},
        device::{
            Endpoint,
            device::{ButtplugDevice, DeviceImplCommand, DeviceWriteCmd},
            configuration_manager::{DeviceSpecifier, BluetoothLESpecifier},
        }
    };
    use async_std::task;

    #[test]
    pub fn test_aneros_protocol() {
        task::block_on(async move {
            let specifier = DeviceSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device("Massage Demo"));
            let device_impl = TestDevice::new("Massage Demo", vec!(Endpoint::Tx));
            let mut device_impl_clone = device_impl.clone();
            let device_impl_creator = TestDeviceImplCreator::new(specifier, Box::new(device_impl));
            let mut device: ButtplugDevice = ButtplugDevice::try_create_device(Box::new(device_impl_creator)).await.unwrap().unwrap();
            device.parse_message(&VibrateCmd::new(0, vec!(VibrateSubcommand::new(0, 0.5))).into()).await.unwrap();
            let command_receiver = device_impl_clone.endpoint_channels.get_mut(&Endpoint::Tx).unwrap().1.clone();
            let command = command_receiver.recv().await.unwrap();
            assert_eq!(command, DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF1, 63], false)));
        });
    }
}
