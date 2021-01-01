use super::xinput_device_impl::XInputDeviceImplCreator;
use crate::{
  core::ButtplugResultFuture,
  server::comm_managers::{
    DeviceCommunicationEvent,
    DeviceCommunicationManager,
    DeviceCommunicationManagerCreator,
  },
  util::async_manager,
};
use async_channel::Sender;
use futures::{future, FutureExt};
use futures_timer::Delay;
use std::{sync::Arc, time::Duration};
use tokio::sync::Notify;

#[derive(Debug, Display, Clone, Copy)]
#[repr(u8)]
pub enum XInputControllerIndex {
  XInputController0 = 0,
  XInputController1 = 1,
  XInputController2 = 2,
  XInputController3 = 3,
}

pub struct XInputDeviceCommunicationManager {
  sender: Sender<DeviceCommunicationEvent>,
  scanning_notifier: Arc<Notify>,
}

impl DeviceCommunicationManagerCreator for XInputDeviceCommunicationManager {
  fn new(sender: Sender<DeviceCommunicationEvent>) -> Self {
    Self {
      sender,
      scanning_notifier: Arc::new(Notify::new()),
    }
  }
}

impl DeviceCommunicationManager for XInputDeviceCommunicationManager {
  fn name(&self) -> &'static str {
    "XInputDeviceCommunicationManager"
  }

  fn start_scanning(&self) -> ButtplugResultFuture {
    info!("XInput manager scanning!");
    let sender = self.sender.clone();
    let scanning_notifier = self.scanning_notifier.clone();
    async_manager::spawn(async move {
      let handle = rusty_xinput::XInputHandle::load_default().unwrap();
      let mut stop = false;
      // On first scan, we'll re-emit all xinput devices. If the system
      // already has them, they'll just be ignored because the addresses
      // will collide. However, this saves us from re-emitting them on
      // EVERY scan during the timed scan loop.
      let mut connected_indexes = vec![];
      while !stop {
        for i in &[
          XInputControllerIndex::XInputController0,
          XInputControllerIndex::XInputController1,
          XInputControllerIndex::XInputController2,
          XInputControllerIndex::XInputController3,
        ] {
          match handle.get_state(*i as u32) {
            Ok(_) => {
              let index = *i as u32;
              if connected_indexes.contains(&index) {
                trace!("XInput device {} already found, ignoring.", *i);
                continue;
              }
              info!("XInput manager found device {}", index);
              let device_creator = Box::new(XInputDeviceImplCreator::new(*i));
              connected_indexes.push(index);
              if sender
                .send(DeviceCommunicationEvent::DeviceFound(device_creator))
                .await
                .is_err()
              {
                error!("Error sending device found message from Xinput.");
                break;
              }
            }
            Err(_) => {
              let index = *i as u32;
              if connected_indexes.contains(&index) {
                info!("XInput device {} disconnected", index);
              }
              connected_indexes.retain(|x| *x != index);
              continue;
            }
          }
        }
        // Wait for either one second, or until our notifier has been notified.
        select! {
          _ = Delay::new(Duration::from_secs(1)).fuse() => {},
          _ = scanning_notifier.notified().fuse() => {
            info!("XInput stop scanning notifier notified, ending scanning loop");
            stop = true;
          }
        }
      }
    })
    .unwrap();
    Box::pin(future::ready(Ok(())))
  }

  fn stop_scanning(&self) -> ButtplugResultFuture {
    debug!("XInput device comm manager received Stop Scanning request");
    self.scanning_notifier.notify_waiters();
    let sender = self.sender.clone();
    Box::pin(async move {
      if sender
        .send(DeviceCommunicationEvent::ScanningFinished)
        .await
        .is_err()
      {
        error!("Error sending scanning finished from Xinput.");
      }
      Ok(())
    })
  }
}
