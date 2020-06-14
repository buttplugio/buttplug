use crate::{
  core::errors::ButtplugDeviceError,
  device::{
    configuration_manager::DeviceProtocolConfiguration, ButtplugDeviceEvent, DeviceSubscribeCmd,
    DeviceUnsubscribeCmd,
  },
};
use futures::future::BoxFuture;
use futures::StreamExt;
use super::{ButtplugProtocol, ButtplugProtocolCommandHandler, ButtplugProtocolCreator};
use crate::{
  core::{
    errors::ButtplugError,
    messages::{self, ButtplugDeviceCommandMessageUnion, MessageAttributesMap},
  },
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl, DeviceWriteCmd, Endpoint,
  },
  server::ButtplugServerResultFuture,
};
use std::cell::RefCell;

#[derive(ButtplugProtocol, ButtplugProtocolProperties)]
pub struct Lovense {
  name: String,
  message_attributes: MessageAttributesMap,
  manager: RefCell<GenericCommandManager>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl Lovense {
  pub(super) fn new(name: &str, message_attributes: MessageAttributesMap) -> Self {
    let manager = GenericCommandManager::new(&message_attributes);

    Self {
      name: name.to_owned(),
      message_attributes,
      stop_commands: manager.get_stop_commands(),
      manager: RefCell::new(manager),
    }
  }
}

impl ButtplugProtocolCreator for Lovense {
  fn new_protocol(name: &str, attrs: MessageAttributesMap) -> Box<dyn ButtplugProtocol> {
    Box::new(Self::new(name, attrs))
  }

  fn try_create(
    device_impl: &dyn DeviceImpl,
    configuration: DeviceProtocolConfiguration,
  ) -> BoxFuture<'static, Result<Box<dyn ButtplugProtocol>, ButtplugError>>
  {
    let subscribe_fut = device_impl.subscribe(DeviceSubscribeCmd::new(Endpoint::Rx));
    let msg = DeviceWriteCmd::new(Endpoint::Tx, b"DeviceType;".to_vec(), false);
    let info_fut = device_impl.write_value(msg);
    let mut event_receiver = device_impl.get_event_receiver();
    let unsubscribe_fut = device_impl.unsubscribe(DeviceUnsubscribeCmd::new(Endpoint::Rx));
    Box::pin(async move {
      let identifier;
      subscribe_fut.await?;
      info_fut.await?;
      // TODO Put some sort of very quick timeout here, we should just fail if
      // we don't get something back quickly.
      match event_receiver.next().await {
        Some(ButtplugDeviceEvent::Notification(_, n)) => {
          let type_response = std::str::from_utf8(&n).unwrap().to_owned();
          info!("Lovense Device Type Response: {}", type_response);
          identifier = type_response.split(':').collect::<Vec<&str>>()[0].to_owned();
        }
        Some(ButtplugDeviceEvent::Removed) => {
          return Err(
            ButtplugDeviceError::new("Lovense Device disconnected while getting DeviceType info.")
              .into(),
          );
        }
        None => {
          return Err(
            ButtplugDeviceError::new("Did not get DeviceType return from Lovense device in time")
              .into(),
          );
        }
      };
      unsubscribe_fut.await?;
      let (names, attrs) = configuration.get_attributes(&identifier).unwrap();
      let name = names.get("en-us").unwrap();
      Ok(Self::new_protocol(name, attrs))
    })
  }
}

impl ButtplugProtocolCommandHandler for Lovense {
  fn handle_vibrate_cmd(
    &self,
    device: &dyn DeviceImpl,
    msg: messages::VibrateCmd,
  ) -> ButtplugServerResultFuture {
    // Store off result before the match, so we drop the lock ASAP.
    let result = self.manager.borrow_mut().update_vibration(&msg, false);
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
    match result {
      Ok(cmds_option) => {
        let mut fut_vec = vec![];
        if let Some(cmds) = cmds_option {
          if !cmds[0].is_none() && (cmds.len() == 1 || cmds.windows(2).all(|w| w[0] == w[1])) {
            let lovense_cmd = format!("Vibrate:{};", cmds[0].unwrap()).as_bytes().to_vec();
            let fut = device.write_value(DeviceWriteCmd::new(Endpoint::Tx, lovense_cmd, false));
            return Box::pin(async {
              fut.await?;
              Ok(messages::Ok::default().into())
            });
          }
          for i in 0..cmds.len() {
            if let Some(speed) = cmds[i] {
              let lovense_cmd = format!("Vibrate{}:{};", i + 1, speed).as_bytes().to_vec();
              fut_vec.push(device.write_value(DeviceWriteCmd::new(
                Endpoint::Tx,
                lovense_cmd,
                false,
              )));
            }
          }
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
  }

  // TODO Reimplement this for futures passing
  /*
  fn handle_rotate_cmd(
    &self,
    device: &dyn DeviceImpl,
    msg: messages::RotateCmd,
  ) -> ButtplugServerResultFuture {
    let result = self.manager.lock().await.update_rotation(msg);
    match result {
      Ok(cmds) => {
        // Due to lovense devices having separate commands for rotation
        // and speed, we can't completely depend on the generic command
        // manager here.
        //
        // TODO Should the generic command manager maybe store the
        // previous command as well as returning the next? That might
        // save us having to store this in the protocol members, but I'm
        // also not sure anyone but Lovense does this. For Vorze, we
        // need speed and direction regardless because they form a
        // single command.
        if let Some((speed, clockwise)) = cmds[0] {
          let mut lovense_cmds = vec![];
          {
            let mut last_rotation = self.last_rotation.lock().await;
            if let Some((rot_speed, rot_dir)) = *last_rotation {
              if rot_dir != clockwise {
                lovense_cmds.push("RotateChange;".as_bytes().to_vec());
              }
              if rot_speed != speed {
                lovense_cmds.push(format!("Rotate:{};", speed).as_bytes().to_vec());
              }
            }
            *last_rotation = Some((speed, clockwise));
          }
          for cmd in lovense_cmds {
            device
              .write_value(DeviceWriteCmd::new(Endpoint::Tx, cmd, false))
              .await?;
          }
        }
        Ok(messages::Ok::default().into())
      }
      Err(e) => Err(e),
    }
  }
  */
}

// TODO Gonna need to add the ability to set subscribe data in tests before
// writing Lovense tests. Oops.
