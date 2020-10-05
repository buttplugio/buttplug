use super::{
  ButtplugDeviceResultFuture,
  ButtplugProtocol,
  ButtplugProtocolCommandHandler,
  ButtplugProtocolCreator,
};
use crate::{
  core::errors::ButtplugDeviceError,
  device::{
    configuration_manager::DeviceProtocolConfiguration,
    ButtplugDeviceEvent,
    DeviceSubscribeCmd,
    BoundedDeviceEventBroadcaster
  },
};
use crate::{
  core::{
    errors::ButtplugError,
    messages::{self, ButtplugDeviceCommandMessageUnion, MessageAttributesMap},
  },
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl,
    DeviceWriteCmd,
    Endpoint,
  },
  util::async_manager
};
use async_mutex::Mutex;
use async_channel::{Receiver, bounded};
use futures::future::BoxFuture;
use futures::StreamExt;
use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc,
};

#[derive(ButtplugProtocol, ButtplugProtocolProperties)]
pub struct Lovense {
  name: String,
  message_attributes: MessageAttributesMap,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
  rotation_direction: Arc<AtomicBool>,
  battery_receiver: Receiver<f64>
}

impl Lovense {
  pub(super) fn new(name: &str, message_attributes: MessageAttributesMap, mut device_notification_receiver: BoundedDeviceEventBroadcaster) -> Self {
    let manager = GenericCommandManager::new(&message_attributes);
    let (battery_sender, battery_receiver) = bounded(256);
    async_manager::spawn(async move {
      while let Some(event) = device_notification_receiver.recv().await {
        match event {
          ButtplugDeviceEvent::Notification(_, data) => {
            if let Ok(data_str) = std::str::from_utf8(&data) {
              let len = data_str.len();
              // Chop the semicolon at the end of the received line.
              if let Ok(level) = data_str[0..(len-1)].parse::<u8>() {
                if let Err(_) = battery_sender.send(level as f64 / 100f64).await {
                  debug!("Lovense device removed, exiting notification loop.");
                  return;
                }
              }
            }
          }
          ButtplugDeviceEvent::Removed => {
            debug!("Lovense device removed, exiting notification loop.");
            return;
          }
        }
      }
      debug!("Lovense notification channel shut down, assuming device has disconnected.");
    }).unwrap();
    Self {
      name: name.to_owned(),
      message_attributes,
      stop_commands: manager.get_stop_commands(),
      manager: Arc::new(Mutex::new(manager)),
      rotation_direction: Arc::new(AtomicBool::new(false)),
      battery_receiver
    }
  }
}

impl ButtplugProtocolCreator for Lovense {
  // Due to this lacking the ability to take extra fields, we can't pass in our
  // event receiver from the subscription, which we'll need for things like
  // battery readings. Therefore, we expect try_create() to return the protocol
  // itself instead of calling this, which is simply a convenience method for
  // the default implementation anyways.
  fn new_protocol(_name: &str, _attrs: MessageAttributesMap) -> Box<dyn ButtplugProtocol> {
    unimplemented!("This should never be called!");
  }

  fn try_create(
    device_impl: &dyn DeviceImpl,
    configuration: DeviceProtocolConfiguration,
  ) -> BoxFuture<'static, Result<Box<dyn ButtplugProtocol>, ButtplugError>> {
    let subscribe_fut = device_impl.subscribe(DeviceSubscribeCmd::new(Endpoint::Rx));
    let msg = DeviceWriteCmd::new(Endpoint::Tx, b"DeviceType;".to_vec(), false);
    let info_fut = device_impl.write_value(msg);
    let mut event_receiver = device_impl.get_event_receiver();
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
            ButtplugDeviceError::ProtocolSpecificError(
              "Lovense",
              "Lovense Device disconnected while getting DeviceType info.",
            )
            .into(),
          );
        }
        None => {
          return Err(
            ButtplugDeviceError::ProtocolSpecificError(
              "Lovense",
              "Did not get DeviceType return from Lovense device in time",
            )
            .into(),
          );
        }
      };
      let (names, attrs) = configuration.get_attributes(&identifier).unwrap();
      let name = names.get("en-us").unwrap();
      let device: Box<dyn ButtplugProtocol> = Box::new(Self::new(name, attrs, event_receiver));
      Ok(device)
    })
  }
}

impl ButtplugProtocolCommandHandler for Lovense {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<Box<dyn DeviceImpl>>,
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
          let lovense_cmd = format!("Vibrate:{};", cmds[0].unwrap()).as_bytes().to_vec();
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
    device: Arc<Box<dyn DeviceImpl>>,
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
            "RotateChange;".as_bytes().to_vec(),
            false,
          ));
          fut.await?;
        }
      }
      Ok(messages::Ok::default().into())
    })
  }

  fn handle_battery_level_cmd(&self, device: Arc<Box<dyn DeviceImpl>>, message: messages::BatteryLevelCmd) -> ButtplugDeviceResultFuture {
    let mut battery_receiver = self.battery_receiver.clone();
    Box::pin(async move {
      let write_fut = device.write_value(DeviceWriteCmd::new(
        Endpoint::Tx,
        "Battery;".as_bytes().to_vec(),
        false,
      ));
      write_fut.await?;
      match battery_receiver.next().await {
        Some(battery_level) => Ok(messages::BatteryLevelReading::new(message.device_index, battery_level).into()),
        None => Err(ButtplugDeviceError::DeviceNotConnected("Notification channel no longer exists, assuming device disconnected.".to_owned()).into())
      }
    })
  }
}

// TODO Gonna need to add the ability to set subscribe data in tests before
// writing Lovense tests. Oops.
