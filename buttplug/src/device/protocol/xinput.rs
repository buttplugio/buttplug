use crate::create_buttplug_protocol;
use byteorder::{LittleEndian, WriteBytesExt};
use crate::core::errors::ButtplugMessageError;

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
        let result = self.manager.lock().await.update_vibration(msg, true);
        // My life for an async closure so I could just do this via and_then(). :(
        match result {
            Ok(cmds_option) => {
                if let Some(cmds) = cmds_option {
                    // XInput is fast enough that we can ignore the commands handed
                    // back by the manager and just form our own packet. This means
                    // we'll just use the manager's return for command validity
                    // checking.
                    let mut cmd = vec![];
                    cmd.write_u16::<LittleEndian>(cmds[0].unwrap() as u16)
                        .map_err(|_| ButtplugError::ButtplugMessageError(ButtplugMessageError::new("Cannot convert XInput value for processing")))?;
                    cmd.write_u16::<LittleEndian>(cmds[1].unwrap() as u16)
                        .map_err(|_| ButtplugError::ButtplugMessageError(ButtplugMessageError::new("Cannot convert XInput value for processing")))?;
                    device
                    .write_value(DeviceWriteCmd::new(
                        Endpoint::Tx,
                        cmd,
                        false,
                    ))
                    .await?;
                }
                Ok(messages::Ok::default().into())
            }
            Err(e) => Err(e),
        }
    }))
);
