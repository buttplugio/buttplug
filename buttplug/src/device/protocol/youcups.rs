use crate::create_buttplug_protocol;

create_buttplug_protocol!(
    // Protocol name
    Youcups,
    // Use the default protocol creator implementation. No special init needed.
    true,
    // No extra members,
    (),
    (
        (VibrateCmd, {
            // TODO Convert to using generic command manager
            let msg = DeviceWriteCmd::new(
                Endpoint::Tx,
                format!("$SYS,{}?", (msg.speeds[0].speed * 8.0) as u8)
                .as_bytes()
                .to_vec(),
                false,
            );
            let fut = device.write_value(msg.into());
            Box::pin(async {
                fut.await?;
                Ok(messages::Ok::default().into())
            })
        })
    )
);

// TODO Write Tests
