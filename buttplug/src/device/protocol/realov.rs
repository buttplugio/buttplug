use crate::create_buttplug_protocol;

create_buttplug_protocol!(Realov,
    (VibrateCmd, {
        // TODO Convert to using generic command manager        
        let speed = (msg.speeds[0].speed * 50.0) as u8;
        let msg = DeviceWriteCmd::new(Endpoint::Tx, [0xc5, 0x55, speed, 0xaa].to_vec(), false);
        device.write_value(msg.into()).await?;
        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    })
);

// TODO Write Tests