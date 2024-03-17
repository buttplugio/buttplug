// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{
  lovense_dongle_messages::{
    LovenseDeviceCommand,
    LovenseDongleIncomingMessage,
    OutgoingLovenseData,
  },
  lovense_dongle_state_machine::create_lovense_dongle_machine,
};
use crate::{
  core::{errors::ButtplugDeviceError, ButtplugResultFuture},
  server::device::hardware::communication::{
    HardwareCommunicationManager,
    HardwareCommunicationManagerBuilder,
    HardwareCommunicationManagerEvent,
  },
  util::async_manager,
};
use futures::FutureExt;
use hidapi::{HidApi, HidDevice};
use serde_json::Deserializer;
use std::{
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  thread,
};
use tokio::{
  runtime,
  sync::{
    mpsc::{channel, Receiver, Sender},
    Mutex,
  },
};
use tokio_util::sync::CancellationToken;
use tracing_futures::Instrument;

fn hid_write_thread(
  dongle: HidDevice,
  mut receiver: Receiver<OutgoingLovenseData>,
  token: CancellationToken,
) {
  info!("Starting HID dongle write thread");
  let rt = runtime::Builder::new_current_thread()
    .build()
    .expect("Should always build");
  let _guard = rt.enter();
  let port_write = |mut data: String| {
    data += "\r\n";
    info!("Writing message: {}", data);

    // For HID, we have to append the null report id before writing.
    let data_bytes = data.into_bytes();
    info!("Writing length: {}", data_bytes.len());
    // We need to keep the first and last byte of our HID report 0, and we're
    // packing 65 bytes (1 report id, 64 bytes data). We can chunk into 63 byte
    // pieces and iterate.
    for chunk in data_bytes.chunks(63) {
      trace!("bytes: {:?}", chunk);
      let mut byte_array = [0u8; 65];
      byte_array[1..chunk.len() + 1].copy_from_slice(chunk);
      if let Err(err) = dongle.write(&byte_array) {
        // We're probably going to exit very quickly after this.
        error!("Cannot write to dongle: {}", err);
      }
    }
  };

  while let Some(data) = rt.block_on(async {
    select! {
      _ = token.cancelled().fuse() => None,
      data = receiver.recv().fuse() => data
    }
  }) {
    match data {
      OutgoingLovenseData::Raw(s) => {
        port_write(s);
      }
      OutgoingLovenseData::Message(m) => {
        port_write(serde_json::to_string(&m).expect("This will always serialize."));
      }
    }
  }
  trace!("Leaving HID dongle write thread");
}

fn hid_read_thread(
  dongle: HidDevice,
  sender: Sender<LovenseDongleIncomingMessage>,
  token: CancellationToken,
) {
  trace!("Starting HID dongle read thread");
  dongle
    .set_blocking_mode(true)
    .expect("Should alwasy succeed.");
  let mut data: String = String::default();
  let mut buf = [0u8; 1024];
  while !token.is_cancelled() {
    match dongle.read_timeout(&mut buf, 100) {
      Ok(len) => {
        if len == 0 {
          continue;
        }
        trace!("Got {} hid bytes", len);
        // Don't read last byte, as it'll always be 0 since the string
        // terminator is sent.
        data += std::str::from_utf8(&buf[0..len - 1])
          .expect("We should at least get strings from the dongle.");
        if data.contains('\n') {
          // We have what should be a full message.
          // Split it.
          let msg_vec: Vec<&str> = data.split('\n').collect();

          let incoming = msg_vec[0];
          let sender_clone = sender.clone();

          let stream = Deserializer::from_str(incoming).into_iter::<LovenseDongleIncomingMessage>();
          for msg in stream {
            match msg {
              Ok(m) => {
                trace!("Read message: {:?}", m);
                if let Err(err) = sender_clone.blocking_send(m) {
                  // Error, assume we'll be cancelled by disconnect.
                  error!(
                    "Error sending message, assuming device disconnect: {:?}",
                    err
                  );
                }
              }
              Err(_e) => {
                //error!("Error reading: {:?}", e);
                /*
                sender_clone
                  .send(IncomingLovenseData::Raw(incoming.clone().to_string()))
                  .await;
                  */
              }
            }
          }
          // Save off the extra.
          data = String::default();
        }
      }
      Err(e) => {
        error!("{:?}", e);
        break;
      }
    }
  }
  trace!("Leaving HID dongle read thread");
}

#[derive(Default, Clone)]
pub struct LovenseHIDDongleCommunicationManagerBuilder {}

impl HardwareCommunicationManagerBuilder for LovenseHIDDongleCommunicationManagerBuilder {
  fn finish(
    &mut self,
    sender: Sender<HardwareCommunicationManagerEvent>,
  ) -> Box<dyn HardwareCommunicationManager> {
    Box::new(LovenseHIDDongleCommunicationManager::new(sender))
  }
}

pub struct LovenseHIDDongleCommunicationManager {
  machine_sender: Sender<LovenseDeviceCommand>,
  read_thread: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
  write_thread: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
  is_scanning: Arc<AtomicBool>,
  thread_cancellation_token: CancellationToken,
  dongle_available: Arc<AtomicBool>,
}

impl LovenseHIDDongleCommunicationManager {
  fn new(event_sender: Sender<HardwareCommunicationManagerEvent>) -> Self {
    trace!("Lovense dongle HID Manager created");
    let (machine_sender, machine_receiver) = channel(256);
    let dongle_available = Arc::new(AtomicBool::new(false));
    let mgr = Self {
      machine_sender,
      read_thread: Arc::new(Mutex::new(None)),
      write_thread: Arc::new(Mutex::new(None)),
      is_scanning: Arc::new(AtomicBool::new(false)),
      thread_cancellation_token: CancellationToken::new(),
      dongle_available,
    };
    let dongle_fut = mgr.find_dongle();
    async_manager::spawn(
      async move {
        let _ = dongle_fut.await;
      }
      .instrument(tracing::info_span!("Lovense HID Dongle Finder Task")),
    );
    let mut machine =
      create_lovense_dongle_machine(event_sender, machine_receiver, mgr.is_scanning.clone());
    async_manager::spawn(
      async move {
        while let Some(next) = machine.transition().await {
          machine = next;
        }
      }
      .instrument(tracing::info_span!("Lovense HID Dongle State Machine")),
    );
    mgr
  }

  fn find_dongle(&self) -> ButtplugResultFuture {
    // First off, see if we can actually find a Lovense dongle. If we already
    // have one, skip on to scanning. If we can't find one, send message to log
    // and stop scanning.

    let machine_sender_clone = self.machine_sender.clone();
    let held_read_thread = self.read_thread.clone();
    let held_write_thread = self.write_thread.clone();
    let read_token = self.thread_cancellation_token.child_token();
    let write_token = self.thread_cancellation_token.child_token();
    let dongle_available = self.dongle_available.clone();
    async move {
      let (writer_sender, writer_receiver) = channel(256);
      let (reader_sender, reader_receiver) = channel(256);
      let api = HidApi::new().map_err(|_| {
        // This may happen if we create a new server in the same process?
        error!("Failed to create HIDAPI instance. Was one already created?");
        ButtplugDeviceError::DeviceConnectionError("Cannot create HIDAPI.".to_owned())
      })?;

      // We can't clone HIDDevices, so instead we just open 2 instances of the same one to pass to
      // the different threads. Ugh.
      let dongle1 = api.open(0x1915, 0x520a).map_err(|_| {
        warn!("Cannot find lovense HID dongle.");
        ButtplugDeviceError::DeviceConnectionError("Cannot find lovense HID Dongle.".to_owned())
      })?;
      let dongle2 = api.open(0x1915, 0x520a).map_err(|_| {
        warn!("Cannot find lovense HID dongle.");
        ButtplugDeviceError::DeviceConnectionError("Cannot find lovense HID Dongle.".to_owned())
      })?;

      dongle_available.store(true, Ordering::SeqCst);

      let read_thread = thread::Builder::new()
        .name("Lovense Dongle HID Reader Thread".to_string())
        .spawn(move || {
          hid_read_thread(dongle1, reader_sender, read_token);
        })
        .expect("Thread should always spawn");

      let write_thread = thread::Builder::new()
        .name("Lovense Dongle HID Writer Thread".to_string())
        .spawn(move || {
          hid_write_thread(dongle2, writer_receiver, write_token);
        })
        .expect("Thread should always spawn");

      *(held_read_thread.lock().await) = Some(read_thread);
      *(held_write_thread.lock().await) = Some(write_thread);
      if machine_sender_clone
        .send(LovenseDeviceCommand::DongleFound(
          writer_sender,
          reader_receiver,
        ))
        .await
        .is_err() {
          warn!("We've already spun up the state machine, this receiver should exist, but if we're shutting down this will throw.");
        }
      info!("Found Lovense HID Dongle");
      Ok(())
    }
    .boxed()
  }

  pub fn scanning_status(&self) -> Arc<AtomicBool> {
    self.is_scanning.clone()
  }
}

impl HardwareCommunicationManager for LovenseHIDDongleCommunicationManager {
  fn name(&self) -> &'static str {
    "LovenseHIDDongleCommunicationManager"
  }

  fn start_scanning(&mut self) -> ButtplugResultFuture {
    debug!("Lovense Dongle Manager scanning for devices");
    let sender = self.machine_sender.clone();
    self.is_scanning.store(true, Ordering::SeqCst);
    async move {
      sender
        .send(LovenseDeviceCommand::StartScanning)
        .await
        .expect("Machine always exists as long as this object does.");
      Ok(())
    }
    .boxed()
  }

  fn stop_scanning(&mut self) -> ButtplugResultFuture {
    let sender = self.machine_sender.clone();
    async move {
      sender
        .send(LovenseDeviceCommand::StopScanning)
        .await
        .expect("Machine always exists as long as this object does.");
      Ok(())
    }
    .boxed()
  }

  fn scanning_status(&self) -> bool {
    self.is_scanning.load(Ordering::SeqCst)
  }

  fn can_scan(&self) -> bool {
    self.dongle_available.load(Ordering::SeqCst)
  }
}

impl Drop for LovenseHIDDongleCommunicationManager {
  fn drop(&mut self) {
    self.thread_cancellation_token.cancel();
  }
}
