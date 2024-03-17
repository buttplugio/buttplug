// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::hardware::communication::HardwareSpecificError,
  server::device::{
    configuration::{ProtocolCommunicationSpecifier, SerialSpecifier},
    hardware::{
      Hardware,
      HardwareConnector,
      HardwareEvent,
      HardwareInternal,
      HardwareReadCmd,
      HardwareReading,
      HardwareSpecializer,
      HardwareSubscribeCmd,
      HardwareUnsubscribeCmd,
      HardwareWriteCmd,
    },
  },
  util::async_manager,
};
use async_trait::async_trait;
use futures::future;
use futures::{future::BoxFuture, FutureExt};
use serialport::{SerialPort, SerialPortInfo};
use std::{
  fmt::{self, Debug},
  io::ErrorKind,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  thread,
  time::Duration,
};
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio_util::sync::CancellationToken;

pub struct SerialPortHardwareConnector {
  specifier: ProtocolCommunicationSpecifier,
  port_info: SerialPortInfo,
}

impl SerialPortHardwareConnector {
  pub fn new(port_info: &SerialPortInfo) -> Self {
    Self {
      specifier: ProtocolCommunicationSpecifier::Serial(SerialSpecifier::new_from_name(
        &port_info.port_name,
      )),
      port_info: port_info.clone(),
    }
  }
}

impl Debug for SerialPortHardwareConnector {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("SerialPortHardwareCreator")
      .field("port_info", &self.port_info)
      .finish()
  }
}

#[async_trait]
impl HardwareConnector for SerialPortHardwareConnector {
  fn specifier(&self) -> ProtocolCommunicationSpecifier {
    self.specifier.clone()
  }

  async fn connect(&mut self) -> Result<Box<dyn HardwareSpecializer>, ButtplugDeviceError> {
    Ok(Box::new(SerialPortHardwareSpecialzier::new(
      &self.port_info,
    )))
  }
}

pub struct SerialPortHardwareSpecialzier {
  port_info: SerialPortInfo,
}

impl SerialPortHardwareSpecialzier {
  pub fn new(port_info: &SerialPortInfo) -> Self {
    Self {
      port_info: port_info.clone(),
    }
  }
}

#[async_trait]
impl HardwareSpecializer for SerialPortHardwareSpecialzier {
  // For serial, connection happens during specialize because we can't find the port until we have
  // the protocol def.
  async fn specialize(
    &mut self,
    specifiers: &[ProtocolCommunicationSpecifier],
  ) -> Result<Hardware, ButtplugDeviceError> {
    let hardware_internal = SerialPortHardware::try_create(&self.port_info, specifiers).await?;
    let hardware = Hardware::new(
      &self.port_info.port_name,
      &self.port_info.port_name,
      &[Endpoint::Rx, Endpoint::Tx],
      Box::new(hardware_internal),
    );
    Ok(hardware)
  }
}

fn serial_write_thread(mut port: Box<dyn SerialPort>, receiver: mpsc::Receiver<Vec<u8>>) {
  let mut recv = receiver;
  // Instead of waiting on a token here, we'll expect that we'll break on our
  // channel going away.
  //
  // This is a blocking recv so we don't have to worry about the port.
  while let Some(v) = recv.blocking_recv() {
    if let Err(err) = port.write_all(&v) {
      error!("Cannot write data to serial port, exiting thread: {}", err);
      return;
    }
  }
}

fn serial_read_thread(
  mut port: Box<dyn SerialPort>,
  sender: mpsc::Sender<Vec<u8>>,
  token: CancellationToken,
) {
  while !token.is_cancelled() {
    // TODO This is probably too small
    let mut buf: [u8; 1024] = [0; 1024];
    match port.bytes_to_read() {
      Ok(read_len) => {
        if read_len == 0 {
          thread::sleep(Duration::from_millis(10));
          continue;
        }
        match port.read(&mut buf) {
          Ok(len) => {
            trace!("Got {} serial bytes", len);
            if sender.blocking_send(buf[0..len].to_vec()).is_err() {
              error!("Serial port implementation disappeared, exiting read thread.");
              break;
            }
          }
          Err(e) => {
            if e.kind() == ErrorKind::TimedOut {
              continue;
            }
            error!("{:?}", e);
          }
        }
      }
      Err(e) => {
        warn!("Error reading from serial port: {:?}", e);
        if e.kind() == serialport::ErrorKind::NoDevice {
          info!("Serial device gone, breaking out of read loop.");
        }
      }
    }
  }
}

pub struct SerialPortHardware {
  address: String,
  port_receiver: Arc<Mutex<mpsc::Receiver<Vec<u8>>>>,
  port_sender: mpsc::Sender<Vec<u8>>,
  connected: Arc<AtomicBool>,
  device_event_sender: broadcast::Sender<HardwareEvent>,
  // TODO These aren't actually read, do we need to hold them?
  _read_thread: thread::JoinHandle<()>,
  _write_thread: thread::JoinHandle<()>,
  _port: Arc<Mutex<Box<dyn SerialPort>>>,
  thread_cancellation_token: CancellationToken,
}

impl SerialPortHardware {
  pub async fn try_create(
    port_info: &SerialPortInfo,
    specifiers: &[ProtocolCommunicationSpecifier],
  ) -> Result<Self, ButtplugDeviceError> {
    let (device_event_sender, _) = broadcast::channel(256);
    // If we've gotten this far, we can expect we have a serial port definition.
    let mut port_def = None;
    for specifier in specifiers {
      if let ProtocolCommunicationSpecifier::Serial(serial) = specifier {
        if port_info.port_name == *serial.port() {
          port_def = Some(serial.clone());
          break;
        }
      }
    }
    let port_def = port_def.expect("We'll always have a port definition by this point");

    // This seems like it should be a oneshot, but there's no way to await a
    // value on those?
    let (port_sender, mut port_receiver) = mpsc::channel(1);
    // Mostly just feeling lazy here and don't wanna do the enum conversions.
    /*
    settings.stop_bits = port_def.stop_bits;
    settings.data_bits = port_def.data_bits;
    settings.parity = port_def.parity;
    */
    // TODO for now, assume 8/N/1. Not really sure when/if this would ever change.
    let port_name = port_info.port_name.clone();
    thread::Builder::new()
      .name("Serial Port Connection Thread".to_string())
      .spawn(move || {
        debug!("Starting serial port connection thread for {}", port_name);
        let port_result = serialport::new(&port_name, *port_def.baud_rate())
          .timeout(Duration::from_millis(100))
          .open();
        if port_sender.blocking_send(port_result)
          .is_err() {
            warn!("Serial port open thread did not return before serial device was dropped. Dropping port.");
          }
        debug!("Exiting serial port connection thread for {}", port_name);
      })
      .expect("Thread creation should always succeed.");

    let port = port_receiver
      .recv()
      .await
      .expect("This will always be a Some value, we're just blocking for bringup")
      .map_err(|e| {
        ButtplugDeviceError::DeviceSpecificError(HardwareSpecificError::SerialError(e.to_string()))
      })?;
    debug!("Serial port received from thread.");
    let (writer_sender, writer_receiver) = mpsc::channel(256);
    let (reader_sender, reader_receiver) = mpsc::channel(256);

    let token = CancellationToken::new();
    let read_token = token.child_token();
    let read_port = (*port)
      .try_clone()
      .expect("Should always be able to clone port");
    let read_thread = thread::Builder::new()
      .name("Serial Reader Thread".to_string())
      .spawn(move || {
        serial_read_thread(read_port, reader_sender, read_token);
      })
      .expect("Should always be able to create thread");

    let write_port = (*port)
      .try_clone()
      .expect("Should always be able to clone port");
    let write_thread = thread::Builder::new()
      .name("Serial Writer Thread".to_string())
      .spawn(move || {
        serial_write_thread(write_port, writer_receiver);
      })
      .expect("Should always be able to create thread");

    Ok(Self {
      address: port
        .name()
        .unwrap_or_else(|| "Default Serial Port Device (No Name Given)".to_owned()),
      _read_thread: read_thread,
      _write_thread: write_thread,
      port_receiver: Arc::new(Mutex::new(reader_receiver)),
      port_sender: writer_sender,
      _port: Arc::new(Mutex::new(port)),
      connected: Arc::new(AtomicBool::new(true)),
      device_event_sender,
      thread_cancellation_token: token,
    })
  }
}

impl HardwareInternal for SerialPortHardware {
  fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.device_event_sender.subscribe()
  }

  fn disconnect(&self) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let connected = self.connected.clone();
    async move {
      connected.store(false, Ordering::SeqCst);
      Ok(())
    }
    .boxed()
  }

  fn read_value(
    &self,
    _msg: &HardwareReadCmd,
  ) -> BoxFuture<'static, Result<HardwareReading, ButtplugDeviceError>> {
    // TODO Should check endpoint validity and length requirements
    let receiver = self.port_receiver.clone();
    async move {
      let mut recv_mut = receiver.lock().await;
      Ok(HardwareReading::new(
        Endpoint::Rx,
        &recv_mut
          .recv()
          .now_or_never()
          .unwrap_or_else(|| Some(vec![]))
          .expect("Always set to Some before unwrapping."),
      ))
    }
    .boxed()
  }

  fn write_value(
    &self,
    msg: &HardwareWriteCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let sender = self.port_sender.clone();
    let data = msg.data.clone();
    // TODO Should check endpoint validity
    async move {
      if sender
        .send(data)
        .await
        .is_err() {
          warn!("Tasks should exist if we get here, but may not if we're shutting down");
        }
      Ok(())
    }
    .boxed()
  }

  fn subscribe(
    &self,
    _msg: &HardwareSubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    // TODO Should check endpoint validity
    let data_receiver = self.port_receiver.clone();
    let event_sender = self.device_event_sender.clone();
    let address = self.address.clone();
    async move {
      async_manager::spawn(async move {
        // TODO There's only one subscribable endpoint on a serial port, so we
        // should check to make sure we don't have multiple subscriptions so we
        // don't deadlock.
        let mut data_receiver_mut = data_receiver.lock().await;
        loop {
          match data_receiver_mut.recv().await {
            Some(data) => {
              info!("Got serial data! {:?}", data);
              event_sender
                .send(HardwareEvent::Notification(
                  address.clone(),
                  Endpoint::Tx,
                  data,
                ))
                .expect("As long as we're subscribed we should have a listener");
            }
            None => {
              info!("Data channel closed, ending serial listener task");
              break;
            }
          }
        }
      });
      Ok(())
    }
    .boxed()
  }

  fn unsubscribe(
    &self,
    _msg: &HardwareUnsubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "Serial port does not support unsubscribe".to_owned(),
    )))
    .boxed()
  }
}

impl Drop for SerialPortHardware {
  fn drop(&mut self) {
    self.thread_cancellation_token.cancel();
  }
}
