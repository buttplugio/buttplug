use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use super::{
  comm_managers::DeviceCommunicationEvent,
  ping_timer::PingTimer
};
use crate::{
  device::{
    configuration_manager::DeviceConfigurationManager,
    ButtplugDevice,
    ButtplugDeviceEvent,
    ButtplugDeviceImplCreator
  },
  core::{
    messages::{ButtplugServerMessage, ScanningFinished, DeviceAdded, DeviceRemoved, StopDeviceCmd},
  },
  util::async_manager
};
use tokio::sync::{mpsc, broadcast, Semaphore};
use dashmap::DashMap;
use futures::{stream::FuturesUnordered, FutureExt, StreamExt};

pub struct DeviceManagerEventLoop {
  device_config_manager: Arc<DeviceConfigurationManager>,
  device_index_generator: u32,
  device_map: Arc<DashMap<u32, ButtplugDevice>>,
  ping_timer: Arc<PingTimer>,
  /// Maps device addresses to indexes, so they can be reused on reconnect.
  device_index_map: Arc<DashMap<String, u32>>,
  /// Broadcaster that relays device events in the form of Buttplug Messages to
  /// whoever owns the Buttplug Server.
  server_sender: broadcast::Sender<ButtplugServerMessage>,
  /// As the device manager owns the Device Communication Managers, it will have
  /// a receiver that the comm managers all send thru.
  device_comm_receiver: mpsc::Receiver<DeviceCommunicationEvent>,
  /// Sender for device events, passed to new devices when they are created.
  device_event_sender: mpsc::Sender<(u32, ButtplugDeviceEvent)>,
  /// Receiver for device events, which the event loops to handle events.
  device_event_receiver: mpsc::Receiver<(u32, ButtplugDeviceEvent)>,
  /// True if StartScanning has been called but no ScanningFinished has been
  /// emitted yet.
  scanning_in_progress: bool,
  /// Holds the status of comm manager scanning states (scanning/not scanning).
  comm_manager_scanning_statuses: Vec<Arc<AtomicBool>>,
  /// Semaphor to make sure we only try to add one device at a time. Some
  /// devices like Lovense can have very odd shutdown issues where they
  /// advertise while powering down, causing glitches in the system as things
  /// try to disconnect/reconnect quickly. Making sure we only insert one device
  /// into our maps at a time saves us from having to reason about this.
  device_addition_semaphore: Arc<Semaphore>,
}

impl DeviceManagerEventLoop {
  pub fn new(
    device_config_manager: Arc<DeviceConfigurationManager>,
    server_sender: broadcast::Sender<ButtplugServerMessage>,
    device_map: Arc<DashMap<u32, ButtplugDevice>>,
    ping_timer: Arc<PingTimer>,
    device_comm_receiver: mpsc::Receiver<DeviceCommunicationEvent>
  ) -> Self {
    let (device_event_sender, device_event_receiver) = mpsc::channel(256);
    Self {
      device_config_manager,
      server_sender,
      device_map,
      ping_timer,
      device_comm_receiver,
      device_index_generator: 0,
      device_index_map: Arc::new(DashMap::new()),
      device_event_sender,
      device_event_receiver,
      scanning_in_progress: false,
      comm_manager_scanning_statuses: vec![],
      device_addition_semaphore: Arc::new(Semaphore::new(1)),
    }
  }

  fn try_create_new_device(&mut self, device_creator: Box<dyn ButtplugDeviceImplCreator>) {
    // Pull and increment the device index now. If connection fails,
    // we'll just iterate to the next one.
    let generated_device_index = self.device_index_generator;
    self.device_index_generator += 1;
    debug!("Current generated device index: {}", generated_device_index);
    let device_event_sender_clone = self.device_event_sender.clone();
    let device_map_clone = self.device_map.clone();
    let server_sender_clone = self.server_sender.clone();
    let device_config_mgr_clone = self.device_config_manager.clone();
    let device_index_map_clone = self.device_index_map.clone();
    let device_addition_semaphore_clone = self.device_addition_semaphore.clone();
    async_manager::spawn(async move {
      match ButtplugDevice::try_create_device(device_config_mgr_clone, device_creator)
        .await
      {
        Ok(option_dev) => match option_dev {
          Some(device) => {
            // In order to make sure we don't collide IDs, we can only
            // insert one device at a time. So much for lockless
            // buttplugs. :(
            //
            // We'll never close this semaphore, so we can unwrap here.
            let _guard = device_addition_semaphore_clone.acquire().await.unwrap();
            // See if we have a reusable device index here.
            let device_index =
              if let Some(id) = device_index_map_clone.get(device.address()) {
                *id.value()
              } else {
                device_index_map_clone
                  .insert(device.address().to_owned(), generated_device_index);
                generated_device_index
              };
            // Since we can now reuse device indexes, this means we
            // might possibly stomp on devices already in the map if
            // they don't register a disconnect before we try to
            // insert the new device. If we have a device already in
            // the map with the same index (and therefore same
            // address), consider it disconnected and eject it from
            // the map. This should also trigger a disconnect event
            // before our new DeviceAdded message goes out, so timing
            // matters here.
            if device_map_clone.contains_key(&device_index) {
              info!("Device map contains key!");
              // We just checked that the key exists, so we can unwrap
              // here.
              let (_, old_device): (_, ButtplugDevice) =
                device_map_clone.remove(&device_index).unwrap();
              // After removing the device from the array, manually
              // disconnect it to make sure the event is thrown.
              if let Err(err) = old_device.disconnect().await {
                // If we throw an error during the disconnect, we
                // can't really do anything with it, but should at
                // least log it.
                error!("Error during index collision disconnect: {:?}", err);
              }
            } else {
              info!("Device map does not contain key!");
            }
            info!("Assigning index {} to {}", device_index, device.name());
            let mut recv = device.get_event_receiver();

            let sender_clone = device_event_sender_clone.clone();
            let idx_clone = device_index;
            // Create a task to forward device events into the device manager
            // event loop.
            async_manager::spawn(async move {
              while let Some(e) = recv.next().await {
                if sender_clone.send((idx_clone, e)).await.is_err() {
                  error!("Device event receiver disappeared, exiting loop.");
                  return;
                }
              }
            })
            .unwrap();

            let device_added_message = DeviceAdded::new(
              device_index,
              &device.name(),
              &device.message_attributes(),
            );
            device_map_clone.insert(device_index, device);
            // After that, we can send out to the server's event
            // listeners to let them know a device has been added.
            if server_sender_clone
              .send(device_added_message.into())
              .is_err()
            {
              error!("Server disappeared, exiting loop.");
              return;
            }
          }
          None => debug!("Device could not be matched to a protocol."),
        },
        Err(e) => error!("Device errored while trying to connect: {}", e),
      }
    })
    .unwrap();
  }

  async fn handle_device_communication(&mut self, event: DeviceCommunicationEvent) {
    match event {
      DeviceCommunicationEvent::ScanningStarted => {
        self.scanning_in_progress = true;
      }
      DeviceCommunicationEvent::ScanningFinished => {
        debug!("System signaled that scanning was finished, check to see if all managers are finished.");
        if !self.scanning_in_progress {
          debug!(
            "Manager finished before scanning was fully started, continuing event loop."
          );
          return;
        }
        if self
          .comm_manager_scanning_statuses
          .iter()
          .any(|x| x.load(Ordering::SeqCst))
        {
          debug!("At least one manager still scanning, continuing event loop.");
          return;
        }
        debug!("All managers finished, emitting ScanningFinished");
        self.scanning_in_progress = false;
        if self
          .server_sender
          .send(ScanningFinished::default().into())
          .is_err()
        {
          error!("Server disappeared, exiting loop.");
          return;
        }
      }
      DeviceCommunicationEvent::DeviceFound(device_creator) => {
        self.try_create_new_device(device_creator);
      }
      DeviceCommunicationEvent::DeviceManagerAdded(status) => {
        self.comm_manager_scanning_statuses.push(status);
      }
    }

  }

  async fn handle_device_event(&mut self, device_index: u32, device_event: ButtplugDeviceEvent) {
    if let ButtplugDeviceEvent::Removed = device_event {
      self.device_map.remove(&device_index);
      if self
        .server_sender
        .send(DeviceRemoved::new(device_index).into())
        .is_err()
      {
        error!("Server disappeared, exiting loop.");
        return;
      }
    }
    info!("Got device event: {:?}", device_event);
  }

  async fn handle_ping_timeout(&self) {
    error!("Pinged out, stopping devices");
    let mut fut_vec = FuturesUnordered::new();
    self
      .device_map
      .iter()
      .for_each(|dev| {
        let device = dev.value();
        fut_vec.push(device.parse_message(StopDeviceCmd::new(1).into()))
      });
    async_manager::spawn(async move {
      while let Some(val) = fut_vec.next().await {
        // Device index doesn't matter here, since we're sending the
        // message directly to the device itself.
        if let Err(e) = val {
          error!("Error stopping device on ping timeout: {}", e);
        }
      }
    }).unwrap();
  }

  pub async fn run(&mut self) {
    loop {
      select! {
        // If we have a ping timeout, stop all devices
        _ = self.ping_timer.ping_timeout_waiter().fuse() => {
          self.handle_ping_timeout().await;
        },
        device_comm_msg = self.device_comm_receiver.recv().fuse() => {
          if let Some(msg) = device_comm_msg {
            self.handle_device_communication(msg).await;
          } else {
            break;
          }
        }
        device_event_msg = self.device_event_receiver.recv().fuse() => {
          if let Some((idx, msg)) = device_event_msg {
            self.handle_device_event(idx, msg).await;
          } else {
            panic!("We shouldn't be able to get here since we also own the sender.");
          }
        },
      }
    }
  }
}
