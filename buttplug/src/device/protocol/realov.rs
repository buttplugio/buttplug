use crate::create_buttplug_protocol;

create_buttplug_protocol!(
    // Protocol name
    Realov,
    // Use the default protocol creator implementation. No special init needed.
    true,
    // No extra members
    (),
    (
        (VibrateCmd, {
            // TODO Convert to using generic command manager
            let speed = (msg.speeds[0].speed * 50.0) as u8;
            let msg = DeviceWriteCmd::new(Endpoint::Tx, [0xc5, 0x55, speed, 0xaa].to_vec(), false);
            device.write_value(msg.into()).await?;
            Ok(messages::Ok::default().into())
        })
    )
);

// TODO Write Tests
