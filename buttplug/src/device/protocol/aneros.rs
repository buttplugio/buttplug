use crate::create_buttplug_protocol;

create_buttplug_protocol!(
    // Protocol name
    Aneros,
    // Use the default protocol creator implementation. No special init needed.
    true,
    // No extra members
    (),
    // Only implements VibrateCmd
    ((VibrateCmd, {
        // Store off result before the match, so we drop the lock ASAP.
        let result = self.manager.lock().await.update_vibration(msg, false);
        // My life for an async closure so I could just do this via and_then(). :(
        match result {
            Ok(cmds_option) => {
                if let Some(cmds) = cmds_option {
                    let mut index = 0u8;
                    for cmd in cmds {
                        if let Some(speed) = cmd {
                            device
                                .write_value(DeviceWriteCmd::new(
                                    Endpoint::Tx,
                                    vec![0xF1 + index, speed as u8],
                                    false,
                                ))
                                .await?;
                        }
                        index += 1;
                    }
                }
                Ok(messages::Ok::default().into())
            }
            Err(e) => Err(e),
        }
    }))
);

#[cfg(test)]
mod test {
    use crate::{
        core::messages::{StopDeviceCmd, VibrateCmd, VibrateSubcommand},
        device::{
            device::{DeviceImplCommand, DeviceWriteCmd},
            Endpoint,
        },
        test::{check_recv_value, TestDevice},
    };
    use async_std::task;

    #[test]
    pub fn test_aneros_protocol() {
        task::block_on(async move {
            let (mut device, test_device) = TestDevice::new_bluetoothle_test_device("Massage Demo")
                .await
                .unwrap();
            device
                .parse_message(&VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
                .await
                .unwrap();
            let (_, command_receiver) = test_device.get_endpoint_channel_clone(&Endpoint::Tx).await;
            check_recv_value(
                &command_receiver,
                DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF1, 63], false)),
            )
            .await;
            // Since we only created one subcommand, we should only receive one command.
            device
                .parse_message(&VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
                .await
                .unwrap();
            assert!(command_receiver.is_empty());
            device
                .parse_message(
                    &VibrateCmd::new(
                        0,
                        vec![
                            VibrateSubcommand::new(0, 0.1),
                            VibrateSubcommand::new(1, 0.5),
                        ],
                    )
                    .into(),
                )
                .await
                .unwrap();
            // TODO There's probably a more concise way to do this.
            check_recv_value(
                &command_receiver,
                DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF1, 12], false)),
            )
            .await;
            check_recv_value(
                &command_receiver,
                DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF2, 63], false)),
            )
            .await;
            device
                .parse_message(&StopDeviceCmd::new(0).into())
                .await
                .unwrap();
            check_recv_value(
                &command_receiver,
                DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF1, 0], false)),
            )
            .await;
            check_recv_value(
                &command_receiver,
                DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF2, 0], false)),
            )
            .await;
        });
    }
}
