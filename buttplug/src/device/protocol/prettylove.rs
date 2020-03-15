use crate::create_buttplug_protocol;

create_buttplug_protocol!(
    // ProtocolName
    PrettyLove,
    // Use the default protocol creator implementation. No special init needed.
    true,
    // No extra members,
    (),
    (
        (VibrateCmd, {
            // TODO Convert to using generic command manager
            let mut speed = (msg.speeds[0].speed * 3.0) as u8;
            if speed == 0 {
                speed = 0xff;
            }
            let msg = DeviceWriteCmd::new(Endpoint::Tx, [0x00, speed].to_vec(), false);
            device.write_value(msg.into()).await?;
            Ok(messages::Ok::default().into())
        })
    )
);

// TODO Write tests