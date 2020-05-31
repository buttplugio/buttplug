use crate::create_buttplug_protocol;
use byteorder::{LittleEndian, WriteBytesExt};

create_buttplug_protocol!(
    // Protocol name
    XInput,
    // Use the default protocol creator implementation. No special init needed.
    true,
    // No extra members
    (),
    // Only implements VibrateCmd
    ((VibrateCmd, {
        // Store off result before the match, so we drop the lock ASAP.
        let result = self.manager.borrow_mut().update_vibration(msg, true);
        // My life for an async closure so I could just do this via and_then(). :(
        match result {
            Ok(cmds_option) => {
                let mut fut_vec = vec!();
                if let Some(cmds) = cmds_option {
                    // XInput is fast enough that we can ignore the commands handed
                    // back by the manager and just form our own packet. This means
                    // we'll just use the manager's return for command validity
                    // checking.
                    let mut cmd = vec![];
                    // TODO Reinstate error handling here, just pass it into the future we hand back.
                    cmd.write_u16::<LittleEndian>(cmds[0].unwrap() as u16).unwrap();
                        //.map_err(|_| ButtplugError::ButtplugMessageError(ButtplugMessageError::new("Cannot convert XInput value for processing")))?;
                    cmd.write_u16::<LittleEndian>(cmds[1].unwrap() as u16).unwrap();
                        //.map_err(|_| ButtplugError::ButtplugMessageError(ButtplugMessageError::new("Cannot convert XInput value for processing")))?;
                    fut_vec.push(device
                    .write_value(DeviceWriteCmd::new(
                        Endpoint::Tx,
                        cmd,
                        false,
                    )));
                }
                Box::pin(async {
                    for fut in fut_vec {
                        fut.await?;
                    }
                    Ok(messages::Ok::default().into())
                })
            }
            Err(e) => e.into(),
        }
    }))
);
