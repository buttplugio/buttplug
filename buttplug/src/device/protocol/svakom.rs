use crate::create_buttplug_protocol;

create_buttplug_protocol!(
    // Protocol name
    Svakom,
    // No extra members
    (),
    (
        (VibrateCmd, {
            // TODO Convert to using generic command manager
            let speed = (msg.speeds[0].speed * 19.0) as u8;
            let multiplier: u8 = if speed == 0x00 { 0x00 } else { 0x01 };
            let msg = DeviceWriteCmd::new(
                Endpoint::Tx,
                [0x55, 0x04, 0x03, 0x00, multiplier, speed].to_vec(),
                false,
            );
            device.write_value(msg.into()).await?;
            Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
        })
    )
);
