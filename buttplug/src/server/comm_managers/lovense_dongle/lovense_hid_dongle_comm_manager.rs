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
  server::comm_managers::{
    DeviceCommunicationEvent,
    DeviceCommunicationManager,
    DeviceCommunicationManagerCreator,
  },
  util::async_manager,
};
use async_channel::{bounded, Receiver, Sender};
use async_lock::Mutex;
use futures::StreamExt;
use hidapi::{HidApi, HidDevice};
use serde_json::Deserializer;
use std::{
  sync::{atomic::AtomicBool, Arc},
  thread,
};
use tracing_futures::Instrument;

fn hid_write_thread(dongle: HidDevice, mut receiver: Receiver<OutgoingLovenseData>) {
  info!("Starting HID dongle write thread");
  let port_write = |mut data: String| {
    data += "\r\n";
    debug!("Writing message: {}", data);

    // For HID, we have to append the null report id before writing.
    let data_bytes = data.into_bytes();
    debug!("Writing length: {}", data_bytes.len());
    // We need to keep the first and last byte of our HID report 0, and we're
    // packing 65 bytes (1 report id, 64 bytes data). We can chunk into 63 byte
    // pieces and iterate.
    for chunk in data_bytes.chunks(63) {
      debug!("bytes: {:?}", chunk);
      let mut byte_array = [0u8; 65];
      byte_array[1..chunk.len() + 1].copy_from_slice(&chunk);
      dongle.write(&byte_array).unwrap();
    }
  };

  while let Some(data) = async_manager::block_on(async { receiver.next().await }) {
    match data {
      OutgoingLovenseData::Raw(s) => {
        port_write(s);
      }
      OutgoingLovenseData::Message(m) => {
        port_write(serde_json::to_string(&m).unwrap());
      }
    }
  }
  info!("Leaving HID dongle write thread");
}

fn hid_read_thread(dongle: HidDevice, sender: Sender<LovenseDongleIncomingMessage>) {
  info!("Starting HID dongle read thread");
  dongle.set_blocking_mode(true).unwrap();
  let mut data: String = String::default();
  let mut buf = [0u8; 1024];
  loop {
    match dongle.read_timeout(&mut buf, 100) {
      Ok(len) => {
        if len == 0 {
          continue;
        }
        debug!("Got {} hid bytes", len);
        // Don't read last byte, as it'll always be 0 since the string
        // terminator is sent.
        data += std::str::from_utf8(&buf[0..len - 1]).unwrap();
        if data.contains('\n') {
          // We have what should be a full message.
          // Split it.
          let msg_vec: Vec<&str> = data.split('\n').collect();

          let incoming = msg_vec[0];
          let sender_clone = sender.clone();

          let stream =
            Deserializer::from_str(&incoming).into_iter::<LovenseDongleIncomingMessage>();
          for msg in stream {
            match msg {
              Ok(m) => {
                debug!("Read message: {:?}", m);
                async_manager::block_on(async { sender_clone.send(m).await }).unwrap();
              }
              Err(e) => {
                error!("Error reading: {:?}", e);
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
  info!("Leaving HID dongle read thread");
}

pub struct LovenseHIDDongleCommunicationManager {
  machine_sender: Sender<LovenseDeviceCommand>,
  read_thread: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
  write_thread: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
}

impl LovenseHIDDongleCommunicationManager {
  fn find_dongle(&self) -> ButtplugResultFuture {
    // First off, see if we can actually find a Lovense dongle. If we already
    // have one, skip on to scanning. If we can't find one, send message to log
    // and stop scanning.

    let machine_sender_clone = self.machine_sender.clone();
    let held_read_thread = self.read_thread.clone();
    let held_write_thread = self.write_thread.clone();
    Box::pin(async move {
      let (writer_sender, writer_receiver) = bounded(256);
      let (reader_sender, reader_receiver) = bounded(256);
      let api = HidApi::new().map_err(|_| {
        // This may happen if we create a new server in the same process?
        error!("Failed to create HIDAPI instance. Was one already created?");
        ButtplugDeviceError::DeviceConnectionError("Cannot create HIDAPI.".to_owned())
      })?;
      let dongle1 = api.open(0x1915, 0x520a).map_err(|_| {
        warn!("Cannot find lovense HID dongle.");
        ButtplugDeviceError::DeviceConnectionError("Cannot find lovense HID Dongle.".to_owned())
      })?;
      let dongle2 = api.open(0x1915, 0x520a).map_err(|_| {
        warn!("Cannot find lovense HID dongle.");
        ButtplugDeviceError::DeviceConnectionError("Cannot find lovense HID Dongle.".to_owned())
      })?;

      let read_thread = thread::Builder::new()
        .name("Lovense Dongle HID Reader Thread".to_string())
        .spawn(move || {
          hid_read_thread(dongle1, reader_sender);
        })
        .unwrap();

      let write_thread = thread::Builder::new()
        .name("Lovense Dongle HID Writer Thread".to_string())
        .spawn(move || {
          hid_write_thread(dongle2, writer_receiver);
        })
        .unwrap();

      *(held_read_thread.lock().await) = Some(read_thread);
      *(held_write_thread.lock().await) = Some(write_thread);
      machine_sender_clone
        .send(LovenseDeviceCommand::DongleFound(
          writer_sender,
          reader_receiver,
        ))
        .await
        .unwrap();
      info!("Found Lovense HID Dongle");
      Ok(())
    })
  }
}

impl DeviceCommunicationManagerCreator for LovenseHIDDongleCommunicationManager {
  fn new(event_sender: Sender<DeviceCommunicationEvent>) -> Self {
    info!("Lovense dongle HID Manager created!");
    let (machine_sender, machine_receiver) = bounded(256);
    let mgr = Self {
      machine_sender,
      read_thread: Arc::new(Mutex::new(None)),
      write_thread: Arc::new(Mutex::new(None)),
    };
    let dongle_fut = mgr.find_dongle();
    async_manager::spawn(
      async move {
        let _ = dongle_fut.await;
      }
      .instrument(tracing::info_span!("Lovense HID Dongle Finder Task")),
    )
    .unwrap();
    async_manager::spawn(
      async move {
        let (mut machine, _) = create_lovense_dongle_machine(event_sender, machine_receiver);
        while let Some(next) = machine.transition().await {
          machine = next;
        }
      }
      .instrument(tracing::info_span!("Lovense HID Dongle State Machine")),
    )
    .unwrap();
    mgr
  }
}

impl DeviceCommunicationManager for LovenseHIDDongleCommunicationManager {
  fn name(&self) -> &'static str {
    "LovenseHIDDongleCommunicationManager"
  }

  fn start_scanning(&self) -> ButtplugResultFuture {
    info!("Lovense Dongle Manager scanning ports!");
    let sender = self.machine_sender.clone();
    Box::pin(async move {
      sender
        .send(LovenseDeviceCommand::StartScanning)
        .await
        .unwrap();
      Ok(())
    })
  }

  fn stop_scanning(&self) -> ButtplugResultFuture {
    let sender = self.machine_sender.clone();
    Box::pin(async move {
      sender
        .send(LovenseDeviceCommand::StopScanning)
        .await
        .unwrap();
      Ok(())
    })
  }

  fn scanning_status(&self) -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
  }
}
