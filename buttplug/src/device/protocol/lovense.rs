use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolCommandHandler};
use crate::{
  core::errors::ButtplugDeviceError,
  device::{ButtplugDeviceEvent, DeviceSubscribeCmd},
};
use crate::{
  core::{
    errors::ButtplugError,
    messages::{
      self,
      ButtplugDeviceCommandMessageUnion,
      ButtplugDeviceMessage,
      DeviceMessageAttributesMap,
    },
  },
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl,
    DeviceWriteCmd,
    Endpoint,
  },
};
use futures::{future::BoxFuture, FutureExt};
use futures_timer::Delay;
use std::{
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  time::Duration,
};
use tokio::sync::Mutex;

// Constants for dealing with the Lovense subscript/write race condition. The
// timeout needs to be VERY long, otherwise this trips up old lovense serial
// adapters.
//
// Just buy new adapters, people.
const LOVENSE_COMMAND_TIMEOUT_MS: u64 = 500;
const LOVENSE_COMMAND_RETRY: u64 = 5;

#[derive(ButtplugProtocolProperties)]
pub struct Lovense {
  name: String,
  message_attributes: DeviceMessageAttributesMap,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
  rotation_direction: Arc<AtomicBool>,
}

impl ButtplugProtocol for Lovense {
  // Due to this lacking the ability to take extra fields, we can't pass in our
  // event receiver from the subscription, which we'll need for things like
  // battery readings. Therefore, we expect initialize() to return the protocol
  // itself instead of calling this, which is simply a convenience method for
  // the default implementation anyways.
  fn new_protocol(name: &str, attrs: DeviceMessageAttributesMap) -> Box<dyn ButtplugProtocol> {
    let manager = GenericCommandManager::new(&attrs);
    Box::new(Self {
      name: name.to_owned(),
      message_attributes: attrs,
      stop_commands: manager.get_stop_commands(),
      manager: Arc::new(Mutex::new(manager)),
      rotation_direction: Arc::new(AtomicBool::new(false)),
    })
  }

  fn initialize(
    device_impl: Arc<DeviceImpl>,
  ) -> BoxFuture<'static, Result<Option<String>, ButtplugError>> {
    Box::pin(async move {
      let mut event_receiver = device_impl.event_stream();
      let identifier;
      let mut count = 0;
      device_impl
        .subscribe(DeviceSubscribeCmd::new(Endpoint::Rx))
        .await?;

      loop {
        let msg = DeviceWriteCmd::new(Endpoint::Tx, b"DeviceType;".to_vec(), false);
        device_impl.write_value(msg).await?;

        select! {
          event = event_receiver.recv().fuse() => {
            if let Ok(ButtplugDeviceEvent::Notification(_, _, n)) = event {
              let type_response = std::str::from_utf8(&n).map_err(|_| ButtplugError::from(ButtplugDeviceError::ProtocolSpecificError("lovense".to_owned(), "Lovense device init got back non-UTF8 string.".to_owned())))?.to_owned();
              info!("Lovense Device Type Response: {}", type_response);
              identifier = type_response.split(':').collect::<Vec<&str>>()[0].to_owned();
              return Ok(Some(identifier));
            } else {
              return Err(
                ButtplugDeviceError::ProtocolSpecificError(
                  "Lovense".to_owned(),
                  "Lovense Device disconnected while getting DeviceType info.".to_owned(),
                )
                .into(),
              );
            }
          }
          _ = Delay::new(Duration::from_millis(LOVENSE_COMMAND_TIMEOUT_MS)).fuse() => {
            count += 1;
            if count > LOVENSE_COMMAND_RETRY {
              return Err(
                ButtplugDeviceError::ProtocolSpecificError(
                  "Lovense".to_owned(),
                  format!("Lovense Device timed out while getting DeviceType info. ({} retries)", LOVENSE_COMMAND_RETRY),
                )
                .into()
              );
            }
          }
        }
      }
    })
  }
}

impl ButtplugProtocolCommandHandler for Lovense {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<DeviceImpl>,
    msg: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    let manager = self.manager.clone();
    Box::pin(async move {
      // Store off result before the match, so we drop the lock ASAP.
      let result = manager.lock().await.update_vibration(&msg, false)?;
      // Lovense is the same situation as the Lovehoney Desire, where commands
      // are different if we're addressing all motors or seperate motors.
      // Difference here being that there's Lovense variants with different
      // numbers of motors.
      //
      // Neat way of checking if everything is the same via
      // https://sts10.github.io/2019/06/06/is-all-equal-function.html.
      //
      // Just make sure we're not matching on None, 'cause if that's the case
      // we ain't got shit to do.
      let mut fut_vec = vec![];
      if let Some(cmds) = result {
        if cmds[0].is_some() && (cmds.len() == 1 || cmds.windows(2).all(|w| w[0] == w[1])) {
          let lovense_cmd = format!("Vibrate:{};", cmds[0].expect("Already checked validity"))
            .as_bytes()
            .to_vec();
          let fut = device.write_value(DeviceWriteCmd::new(Endpoint::Tx, lovense_cmd, false));
          fut.await?;
          return Ok(messages::Ok::default().into());
        }
        for (i, cmd) in cmds.iter().enumerate() {
          if let Some(speed) = cmd {
            let lovense_cmd = format!("Vibrate{}:{};", i + 1, speed).as_bytes().to_vec();
            fut_vec.push(device.write_value(DeviceWriteCmd::new(Endpoint::Tx, lovense_cmd, false)));
          }
        }
      }
      for fut in fut_vec {
        fut.await?;
      }
      Ok(messages::Ok::default().into())
    })
  }

  fn handle_rotate_cmd(
    &self,
    device: Arc<DeviceImpl>,
    msg: messages::RotateCmd,
  ) -> ButtplugDeviceResultFuture {
    let manager = self.manager.clone();
    let direction = self.rotation_direction.clone();
    Box::pin(async move {
      let result = manager.lock().await.update_rotation(&msg)?;
      if let Some((speed, clockwise)) = result[0] {
        let lovense_cmd = format!("Rotate:{};", speed).as_bytes().to_vec();
        let fut = device.write_value(DeviceWriteCmd::new(Endpoint::Tx, lovense_cmd, false));
        fut.await?;
        let dir = direction.load(Ordering::SeqCst);
        // TODO Should we store speed and direction as an option for rotation caching? This is weird.
        if dir != clockwise {
          direction.store(clockwise, Ordering::SeqCst);
          let fut = device.write_value(DeviceWriteCmd::new(
            Endpoint::Tx,
            b"RotateChange;".to_vec(),
            false,
          ));
          fut.await?;
        }
      }
      Ok(messages::Ok::default().into())
    })
  }

  fn handle_battery_level_cmd(
    &self,
    device: Arc<DeviceImpl>,
    message: messages::BatteryLevelCmd,
  ) -> ButtplugDeviceResultFuture {
    let mut device_notification_receiver = device.event_stream();
    Box::pin(async move {
      let write_fut = device.write_value(DeviceWriteCmd::new(
        Endpoint::Tx,
        b"Battery;".to_vec(),
        false,
      ));
      write_fut.await?;
      while let Ok(event) = device_notification_receiver.recv().await {
        match event {
          ButtplugDeviceEvent::Notification(_, _, data) => {
            if let Ok(data_str) = std::str::from_utf8(&data) {
              debug!("Lovense event received: {}", data_str);
              let len = data_str.len();
              // Depending on the state of the toy, we may get an initial
              // character of some kind, i.e. if the toy is currently vibrating
              // then battery level comes up as "s89;" versus just "89;". We'll
              // need to chop the semicolon and make sure we only read the
              // numbers in the string.
              //
              // Contains() is casting a wider net than we need here, but it'll
              // do for now.
              let start_pos = if data_str.contains('s') { 1 } else { 0 };
              if let Ok(level) = data_str[start_pos..(len - 1)].parse::<u8>() {
                return Ok(
                  messages::BatteryLevelReading::new(message.device_index(), level as f64 / 100f64)
                    .into(),
                );
              }
            }
          }
          ButtplugDeviceEvent::Removed(_) => {
            return Err(
              ButtplugDeviceError::ProtocolSpecificError(
                "Lovense".to_owned(),
                "Lovense Device disconnected while getting Battery info.".to_owned(),
              )
              .into(),
            )
          }
          ButtplugDeviceEvent::Connected(_) => {
            unimplemented!("Shouldn't get here as device will always be connected.");
          }
        }
      }
      Err(
        ButtplugDeviceError::ProtocolSpecificError(
          "Lovense".to_owned(),
          "Lovense Device disconnected while getting Battery info.".to_owned(),
        )
        .into(),
      )
    })
  }
}

// TODO Gonna need to add the ability to set subscribe data in tests before
// writing Lovense tests. Oops.
