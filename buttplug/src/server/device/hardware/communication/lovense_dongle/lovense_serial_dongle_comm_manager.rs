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
  core::ButtplugResultFuture,
  server::device::hardware::communication::{
    HardwareCommunicationManager,
    HardwareCommunicationManagerBuilder,
    HardwareCommunicationManagerEvent,
  },
  util::async_manager,
};
use futures::FutureExt;
use serde_json::Deserializer;
use serialport::{available_ports, SerialPort, SerialPortType};
use std::{
  io::ErrorKind,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  thread,
  time::Duration,
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

fn serial_write_thread(
  mut port: Box<dyn SerialPort>,
  mut receiver: Receiver<OutgoingLovenseData>,
  token: CancellationToken,
) {
  let rt = runtime::Builder::new_current_thread()
    .build()
    .expect("Should always build");
  let _guard = rt.enter();

  let mut port_write = |mut data: String| {
    data += "\r\n";
    debug!("Writing message: {}", data);

    // TODO WRITE SHOULD ALWAYS BE FOLLOWED BY A READ UNLESS "EAGER" IS USED
    //
    // We should check this on the outgoing message. Otherwise we will run into
    // all sorts of trouble.
    if let Err(e) = port.write_all(&data.into_bytes()) {
      error!("Cannot write to port: {}", e);
    }
  };
  while let Some(data) = async_manager::block_on(async {
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
        port_write(
          serde_json::to_string(&m).expect("We create these packets so they'll always serialize."),
        );
      }
    }
  }
  debug!("Exiting lovense dongle write thread.");
}

fn serial_read_thread(
  mut port: Box<dyn SerialPort>,
  sender: Sender<LovenseDongleIncomingMessage>,
  token: CancellationToken,
) {
  let mut data: String = String::default();
  while !token.is_cancelled() {
    let mut buf: [u8; 1024] = [0; 1024];
    match port.read(&mut buf) {
      Ok(len) => {
        debug!("Got {} serial bytes", len);
        data += std::str::from_utf8(&buf[0..len])
          .expect("We should always get valid data from the port.");
        if data.contains('\n') {
          debug!("Serial Buffer: {}", data);

          let sender_clone = sender.clone();
          let stream = Deserializer::from_str(&data).into_iter::<LovenseDongleIncomingMessage>();
          for msg in stream {
            match msg {
              Ok(m) => {
                debug!("Read message: {:?}", m);
                async_manager::block_on(async {
                  sender_clone
                    .send(m)
                    .await
                    .expect("Thread shouldn't be running if we don't have a listener.")
                });
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

          // TODO We don't seem to have an extra coming through at the moment,
          // but might need this later?
          data = String::default();
        }
      }
      Err(e) => {
        if e.kind() == ErrorKind::TimedOut {
          continue;
        }
        error!("{:?}", e);
        break;
      }
    }
  }
  debug!("Exiting lovense dongle read thread.");
}

#[derive(Default, Clone)]
pub struct LovenseSerialDongleCommunicationManagerBuilder {}

impl HardwareCommunicationManagerBuilder for LovenseSerialDongleCommunicationManagerBuilder {
  fn finish(
    &mut self,
    sender: Sender<HardwareCommunicationManagerEvent>,
  ) -> Box<dyn HardwareCommunicationManager> {
    Box::new(LovenseSerialDongleCommunicationManager::new(sender))
  }
}

pub struct LovenseSerialDongleCommunicationManager {
  machine_sender: Sender<LovenseDeviceCommand>,
  //port: Arc<Mutex<Option<Box<dyn SerialPort>>>>,
  read_thread: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
  write_thread: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
  is_scanning: Arc<AtomicBool>,
  thread_cancellation_token: CancellationToken,
  dongle_available: Arc<AtomicBool>,
}

impl LovenseSerialDongleCommunicationManager {
  fn new(event_sender: Sender<HardwareCommunicationManagerEvent>) -> Self {
    trace!("Lovense dongle serial port created");
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
    // TODO If we don't find a dongle before scanning, what happens?
    async_manager::spawn(async move {
      if let Err(err) = dongle_fut.await {
        error!("Error finding serial dongle: {:?}", err);
      }
    });
    let mut machine =
      create_lovense_dongle_machine(event_sender, machine_receiver, mgr.is_scanning.clone());
    async_manager::spawn(
      async move {
        while let Some(next) = machine.transition().await {
          machine = next;
        }
      }
      .instrument(tracing::info_span!(
        parent: tracing::Span::current(),
        "Lovense Serial Dongle State Machine"
      )),
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
    let token = self.thread_cancellation_token.child_token();
    let dongle_available = self.dongle_available.clone();
    async move {
      // TODO Does this block? Should it run in one of our threads?
      let found_dongle = false;
      match available_ports() {
        Ok(ports) => {
          debug!("Got {} serial ports back", ports.len());
          for p in ports {
            if let SerialPortType::UsbPort(usb_info) = p.port_type {
              // Hardcode the dongle VID/PID for now. We can't really do protocol
              // detection here because this is a comm bus to us, not a device.
              if usb_info.vid == 0x1a86 && usb_info.pid == 0x7523 {
                // We've found a dongle.
                info!("Found lovense dongle, connecting");
                let serial_port =
                  serialport::new(&p.port_name, 115200).timeout(Duration::from_millis(500));
                match serial_port.open() {
                  Ok(dongle_port) => {
                    let read_token = token.child_token();
                    let write_token = token.child_token();
                    let (writer_sender, writer_receiver) = channel(256);
                    let (reader_sender, reader_receiver) = channel(256);
                    let read_port = (*dongle_port)
                      .try_clone()
                      .expect("USB port should always clone.");
                    let read_thread = thread::Builder::new()
                      .name("Serial Reader Thread".to_string())
                      .spawn(move || {
                        serial_read_thread(read_port, reader_sender, read_token);
                      })
                      .expect("Thread should always create");
                    let write_port = (*dongle_port)
                      .try_clone()
                      .expect("USB port should always clone.");
                    let write_thread = thread::Builder::new()
                      .name("Serial Writer Thread".to_string())
                      .spawn(move || {
                        serial_write_thread(write_port, writer_receiver, write_token);
                      })
                      .expect("Thread should always create");
                    *(held_read_thread.lock().await) = Some(read_thread);
                    *(held_write_thread.lock().await) = Some(write_thread);
                    dongle_available.store(true, Ordering::SeqCst);
                    machine_sender_clone
                      .send(LovenseDeviceCommand::DongleFound(
                        writer_sender,
                        reader_receiver,
                      ))
                      .await
                      .expect("Machine exists if we got here.");
                  }
                  Err(e) => error!("{:?}", e),
                };
              }
            }
          }
        }
        Err(_) => {
          info!("No serial ports found");
        }
      }
      if !found_dongle {
        warn!("Cannot find Lovense Serial dongle.");
      }
      Ok(())
    }
    .instrument(tracing::info_span!("Lovense Serial Dongle Finder"))
    .boxed()
  }
}

impl HardwareCommunicationManager for LovenseSerialDongleCommunicationManager {
  fn name(&self) -> &'static str {
    "LovenseSerialDongleCommunicationManager"
  }

  fn start_scanning(&mut self) -> ButtplugResultFuture {
    debug!("Lovense Dongle Manager scanning for devices.");
    let sender = self.machine_sender.clone();
    async move {
      sender
        .send(LovenseDeviceCommand::StartScanning)
        .await
        .expect("If we're getting scan requests, we should a task to throw it at.");
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
        .expect("If we're getting scan requests, we should a task to throw it at.");
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

impl Drop for LovenseSerialDongleCommunicationManager {
  fn drop(&mut self) {
    self.thread_cancellation_token.cancel();
  }
}
