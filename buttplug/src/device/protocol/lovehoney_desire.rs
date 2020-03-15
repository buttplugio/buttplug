use crate::create_buttplug_protocol;

create_buttplug_protocol!(
    LovehoneyDesire,
    true,
    (),
    (
        (VibrateCmd, {
            // Store off result before the match, so we drop the lock ASAP.
            let result = self.manager.lock().await.update_vibration(msg);

            match result {
                Ok(cmds) => {
                    // The Lovehoney Desire has 2 types of commands
                    //
                    // - Set both motors with one command
                    // - Set each motor separately
                    //
                    // We'll need to check what we got back and write our
                    // commands accordingly.
                    //
                    // Neat way of checking if everything is the same via
                    // https://sts10.github.io/2019/06/06/is-all-equal-function.html.
                    //
                    // Just make sure we're not matching on None, 'cause if
                    // that's the case we ain't got shit to do.
                    if !cmds[0].is_none() && cmds.windows(2).all(|w| w[0] == w[1]) {
                        device.write_value(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF3, 0, cmds[0].unwrap() as u8], false)).await?;
                        return Ok(messages::Ok::default().into());
                    }
                    // We have differening values. Set each motor separately.
                    let mut i = 1;
                    for cmd in cmds {
                        if let Some(speed) = cmd {
                            device.write_value(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF3, i, speed as u8], false)).await?;
                        }
                        i += 1;
                    }

                    Ok(messages::Ok::default().into())
                },
                Err(e) => Err(e)
            }
        }
    ))
);

#[cfg(test)]
mod test {
    use crate::{
        core::messages::{VibrateCmd, VibrateSubcommand, StopDeviceCmd},
        test::{TestDevice, check_recv_value},
        device::{
            Endpoint,
            device::{DeviceImplCommand, DeviceWriteCmd},
        }
    };
    use async_std::task;


    #[test]
    pub fn test_lovehoney_desire_protocol() {
        task::block_on(async move {
            let (mut device, test_device) = TestDevice::new_bluetoothle_test_device("PROSTATE VIBE").await.unwrap();
            let (_, command_receiver) = test_device.get_endpoint_channel_clone(&Endpoint::Tx).await;

            // If we send one speed to one motor, we should only see one output.
            device.parse_message(&VibrateCmd::new(0, vec!(VibrateSubcommand::new(0, 0.5))).into()).await.unwrap();
            check_recv_value(&command_receiver, DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF3, 0x1, 0x3f], false))).await;
            assert!(command_receiver.is_empty());

            // If we send the same speed to each motor, we should only get one command.
            device.parse_message(&VibrateCmd::new(0, vec!(VibrateSubcommand::new(0, 0.1), VibrateSubcommand::new(1, 0.1))).into()).await.unwrap();
            check_recv_value(&command_receiver, DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF3, 0x0, 0x0c], false))).await;
            assert!(command_receiver.is_empty());

            // If we send different commands to both motors, we should get 2 different commands, each with an index.
            device.parse_message(&VibrateCmd::new(0, vec!(VibrateSubcommand::new(0, 0.0), VibrateSubcommand::new(1, 0.5))).into()).await.unwrap();
            check_recv_value(&command_receiver, DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF3, 0x01, 0x00], false))).await;
            check_recv_value(&command_receiver, DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF3, 0x02, 0x3f], false))).await;
            assert!(command_receiver.is_empty());

            device.parse_message(&StopDeviceCmd::new(0).into()).await.unwrap();
            check_recv_value(&command_receiver, DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF3, 0x02, 0x0], false))).await;
            assert!(command_receiver.is_empty());
        });
    }
}
