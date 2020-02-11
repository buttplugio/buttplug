use crate::{
   create_buttplug_protocol
};

create_buttplug_protocol!(Picobong, 
    (VibrateCmd, {
        let speed = (msg.speeds[0].speed * 10.0) as u8;
        let mode: u8 = if speed == 0 { 0xff } else { 0x01 };
        let msg = DeviceWriteCmd::new(Endpoint::Tx, [0x01, mode, speed].to_vec(), false);
        device.write_value(msg.into()).await?;
        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    })
);

// TODO Write tests for protocol