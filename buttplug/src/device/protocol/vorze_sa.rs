use crate::create_buttplug_protocol;

#[repr(u8)]
enum VorzeDevices {
    Bach = 6,
    UFO = 2,
    Cyclone = 1,
}

#[repr(u8)]
enum VorzeActions {
    Rotate = 1,
    Vibrate = 3,
}

create_buttplug_protocol!(
    // Protocol Name
    VorzeSA,
    // Use the default protocol creator implementation. No special init needed.
    true,
    // No special members,
    (),
    (
        (VibrateCmd, {
            let result = self.manager.lock().await.update_vibration(msg);
            match result {
                Ok(cmds) => {
                    if let Some(speed) = cmds[0] {
                        device.write_value(DeviceWriteCmd::new(Endpoint::Tx, vec![VorzeDevices::Bach as u8, VorzeActions::Vibrate as u8, speed as u8], false).into()).await?;
                    }
                    Ok(messages::Ok::default().into())
                },
                Err(e) => Err(e)
            }
        }),
        (RotateCmd, {
            let result = self.manager.lock().await.update_rotation(msg);
            match result {
                Ok(cmds) => {
                    if let Some((speed, clockwise)) = cmds[0] {
                        let dev_id = if self.name.contains("UFO") { VorzeDevices::UFO } else { VorzeDevices::Cyclone };
                        let data: u8 = (clockwise as u8) << 7 | (speed as u8);
                        device.write_value(DeviceWriteCmd::new(Endpoint::Tx, vec![dev_id as u8, VorzeActions::Rotate as u8, data], false).into()).await?;
                    }
                    Ok(messages::Ok::default().into())
                },
                Err(e) => Err(e)
            }
        })
    )
);

#[cfg(test)]
mod test {
    use crate::{
        core::messages::{VibrateCmd, VibrateSubcommand, StopDeviceCmd, RotateCmd, RotationSubcommand},
        test::{TestDevice, check_recv_value},
        device::{
            Endpoint,
            device::{DeviceImplCommand, DeviceWriteCmd},
        }
    };
    use async_std::task;

    #[test]
    pub fn test_vorze_sa_vibration_protocol() {
        task::block_on(async move {
            let (mut device, test_device) = TestDevice::new_bluetoothle_test_device("Bach smart").await.unwrap();
            let (_, command_receiver) = test_device.get_endpoint_channel_clone(&Endpoint::Tx).await;
            device.parse_message(&VibrateCmd::new(0, vec!(VibrateSubcommand::new(0, 0.5))).into()).await.unwrap();
            check_recv_value(&command_receiver, DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0x06, 0x03, 50], false))).await;
            assert!(command_receiver.is_empty());

            device.parse_message(&StopDeviceCmd::new(0).into()).await.unwrap();
            check_recv_value(&command_receiver, DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0x06, 0x03, 0x0], false))).await;
            assert!(command_receiver.is_empty());
        });
    }

    #[test]
    pub fn test_vorze_sa_rotation_protocol() {
        task::block_on(async move {
            let (mut device, test_device) = TestDevice::new_bluetoothle_test_device("CycSA").await.unwrap();
            let (_, command_receiver) = test_device.get_endpoint_channel_clone(&Endpoint::Tx).await;
            device.parse_message(&RotateCmd::new(0, vec!(RotationSubcommand::new(0, 0.5, false))).into()).await.unwrap();
            check_recv_value(&command_receiver, DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0x01, 0x01, 49], false))).await;
            assert!(command_receiver.is_empty());

            device.parse_message(&RotateCmd::new(0, vec!(RotationSubcommand::new(0, 0.5, true))).into()).await.unwrap();
            check_recv_value(&command_receiver, DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0x01, 0x01, 177], false))).await;
            assert!(command_receiver.is_empty());

            device.parse_message(&StopDeviceCmd::new(0).into()).await.unwrap();
            check_recv_value(&command_receiver, DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0x01, 0x01, 0x0], false))).await;
            assert!(command_receiver.is_empty());
        });
    }
}
