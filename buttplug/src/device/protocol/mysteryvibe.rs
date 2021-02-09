use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolCommandHandler};
use crate::{
  core::{
    errors::ButtplugError,
    messages::{self, ButtplugDeviceCommandMessageUnion, DeviceMessageAttributesMap},
  },
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl,
    DeviceWriteCmd,
    Endpoint,
  },
  util::async_manager,
};
use futures::future::BoxFuture;
use futures_timer::Delay;
use std::{
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  time::Duration,
};
use tokio::sync::{Mutex, RwLock};

// Time between Mysteryvibe update commands, in milliseconds. This is basically
// a best guess derived from watching packet timing a few years ago.
//
// Thelemic vibrator. Neat.
//
const MYSTERYVIBE_COMMAND_DELAY_MS: u64 = 93;

#[derive(ButtplugProtocolProperties)]
pub struct MysteryVibe {
  name: String,
  message_attributes: DeviceMessageAttributesMap,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
  current_command: Arc<RwLock<Vec<u8>>>,
  updater_running: Arc<AtomicBool>,
}

impl ButtplugProtocol for MysteryVibe {
  fn new_protocol(
    name: &str,
    message_attributes: DeviceMessageAttributesMap,
  ) -> Box<dyn ButtplugProtocol> {
    let manager = GenericCommandManager::new(&message_attributes);

    Box::new(Self {
      name: name.to_owned(),
      message_attributes,
      stop_commands: manager.get_stop_commands(),
      manager: Arc::new(Mutex::new(manager)),
      updater_running: Arc::new(AtomicBool::new(false)),
      current_command: Arc::new(RwLock::new(vec![0u8, 0, 0, 0, 0, 0])),
    })
  }

  fn initialize(
    device_impl: Arc<DeviceImpl>,
  ) -> BoxFuture<'static, Result<Option<String>, ButtplugError>> {
    let msg = DeviceWriteCmd::new(Endpoint::TxMode, vec![0x43u8, 0x02u8, 0x00u8], true);
    let info_fut = device_impl.write_value(msg);
    Box::pin(async move {
      info_fut.await?;
      Ok(None)
    })
  }
}

async fn vibration_update_handler(device: Arc<DeviceImpl>, command_holder: Arc<RwLock<Vec<u8>>>) {
  info!("Entering Mysteryvibe Control Loop");
  let mut current_command = command_holder.read().await.clone();
  while device
    .write_value(DeviceWriteCmd::new(
      Endpoint::TxVibrate,
      current_command,
      false,
    ))
    .await
    .is_ok()
  {
    Delay::new(Duration::from_millis(MYSTERYVIBE_COMMAND_DELAY_MS)).await;
    current_command = command_holder.read().await.clone();
    info!("MV Command: {:?}", current_command);
  }
  info!("Mysteryvibe control loop exiting, most likely due to device disconnection.");
}

impl ButtplugProtocolCommandHandler for MysteryVibe {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<DeviceImpl>,
    message: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    let manager = self.manager.clone();
    let current_command = self.current_command.clone();
    let update_running = self.updater_running.clone();
    Box::pin(async move {
      let result = manager.lock().await.update_vibration(&message, true)?;
      info!("MV Result: {:?}", result);
      if result.is_none() {
        return Ok(messages::Ok::default().into());
      }
      let write_mutex = current_command.clone();
      let mut command_writer = write_mutex.write().await;
      let command: Vec<u8> = result
        .unwrap()
        .into_iter()
        .map(|x| x.unwrap() as u8)
        .collect();
      *command_writer = command;
      if !update_running.load(Ordering::SeqCst) {
        async_manager::spawn(
          async move { vibration_update_handler(device, current_command).await },
        )
        .unwrap();
        update_running.store(true, Ordering::SeqCst);
      }
      Ok(messages::Ok::default().into())
    })
  }
}

// TODO Write some tests!
//
// At least, once I figure out how to do that with the weird timing on this
// thing.
