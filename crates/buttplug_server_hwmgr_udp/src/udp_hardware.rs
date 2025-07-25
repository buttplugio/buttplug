// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use async_trait::async_trait;
use buttplug_core::{errors::ButtplugDeviceError, util::async_manager};
use buttplug_server::device::hardware::{
  communication::HardwareSpecificError,
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
};
use buttplug_server_device_config::{ProtocolCommunicationSpecifier, UdpSpecifier, Endpoint};
use futures::future;
use futures::{future::BoxFuture, FutureExt};
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
use tokio::net::UdpSocket;
use tokio_util::sync::CancellationToken;

pub struct UdpHardwareConnector {
  specifier: ProtocolCommunicationSpecifier,
}

impl UdpHardwareConnector {
  pub fn new(address: String, port: u16) -> Self {
    Self {
      specifier: ProtocolCommunicationSpecifier::Udp(UdpSpecifier::new(
        &address,
        port
      )),
    }
  }
}

impl Debug for UdpHardwareConnector {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("UdpHardwareConnector")
      .finish()
  }
}

#[async_trait]
impl HardwareConnector for UdpHardwareConnector {
  fn specifier(&self) -> ProtocolCommunicationSpecifier {
    self.specifier.clone()
  }

  async fn connect(&mut self) -> Result<Box<dyn HardwareSpecializer>, ButtplugDeviceError> {
    Ok(Box::new(UdpHardwareSpecialzier::new()))
  }
}

pub struct UdpHardwareSpecialzier {}

impl UdpHardwareSpecialzier {
  pub fn new() -> Self {
    Self {}
  }
}

#[async_trait]
impl HardwareSpecializer for UdpHardwareSpecialzier {
  // For udp, connection happens during specialize because we can't find the port until we have
  // the protocol def.
  async fn specialize(
    &mut self,
    specifiers: &[ProtocolCommunicationSpecifier],
  ) -> Result<Hardware, ButtplugDeviceError> {
    let hardware_internal = UdpHardware::try_create(specifiers).await?;
    let hardware = Hardware::new(
      &hardware_internal.name(),
      &hardware_internal.name(),
      &[Endpoint::Rx, Endpoint::Tx],
      &None,
      false,
      Box::new(hardware_internal),
    );
    Ok(hardware)
  }
}

async fn udp_write_thread(mut socket: Arc<UdpSocket>, receiver: mpsc::Receiver<Vec<u8>>) {
  let mut recv = receiver;
  // Instead of waiting on a token here, we'll expect that we'll break on our
  // channel going away.
  //
  // This is a blocking recv so we don't have to worry about the port.

  while let Some(v) = recv.blocking_recv() {
    if let Err(err) = socket.send(&v).await {
      warn!("Cannot write data to serial port, exiting thread: {}", err);
      return;
    }
  }
}

pub struct UdpHardware {
  address: String,
  port: u16,
  socket_sender: mpsc::Sender<Vec<u8>>,
  connected: Arc<AtomicBool>,
  device_event_sender: broadcast::Sender<HardwareEvent>,
  // TODO These aren't actually read, do we need to hold them?
  _write_thread: thread::JoinHandle<()>,
  _socket: Arc<UdpSocket>,
}

impl UdpHardware {
  pub fn name(&self) -> String {
    format!("{}:{}", self.address, self.port)
  }

  pub async fn try_create(
    specifiers: &[ProtocolCommunicationSpecifier],
  ) -> Result<Self, ButtplugDeviceError> {
    let (device_event_sender, _) = broadcast::channel(256);
    // If we've gotten this far, we can expect we have a socket definition.
    let mut socket_def = None;
    for specifier in specifiers {
      if let ProtocolCommunicationSpecifier::Udp(udp) = specifier {
          socket_def = Some(udp.clone());
          break;
      }
    }
    let socket_def = socket_def.expect("We'll always have a socket definition by this point");

    let (socket_sender, mut socket_receiver) = mpsc::channel(1);
    let name = socket_def.to_string();
    let name_clone = name.clone();
    
    async_manager::spawn(async move {
      debug!("Starting udp socket connection thread for {}", name_clone);
      let socket_result = UdpSocket::bind("0.0.0.0:0").await;
      if socket_sender.blocking_send(socket_result)
        .is_err() {
          warn!("Socket open thread did not return before udp was dropped. Dropping port.");
        }
      debug!("Exiting udp socket connection thread for {}", name_clone);
    });

    let socket = Arc::new(socket_receiver
      .recv()
      .await
      .expect("This will always be a Some value, we're just blocking for bringup")
      .map_err(|e| {
        ButtplugDeviceError::DeviceSpecificError(
          HardwareSpecificError::HardwareSpecificError("Udp".to_owned(), e.to_string())
            .to_string(),
        )
      })?);
    debug!("UDP Socket received from thread.");
    socket.connect(name.clone())
      .await
      .map_err(|e| {
        ButtplugDeviceError::DeviceSpecificError(
          HardwareSpecificError::HardwareSpecificError("UDP".to_owned(), e.to_string()).to_string(),
        )
    })?;

    let (writer_sender, writer_receiver) = mpsc::channel(256);

    let connected = Arc::new(AtomicBool::new(true));
    let connected_clone = connected.clone();
    let event_stream_clone = device_event_sender.clone();
    let write_socket = socket.clone();
    let write_thread = thread::Builder::new()
      .name("Serial Writer Thread".to_string())
      .spawn(move || {
        udp_write_thread(write_socket, writer_receiver);
      })
      .expect("Should always be able to create thread");

    Ok(Self {
      address: name.to_owned(),
      port: *socket_def.port(),
      _write_thread: write_thread,
      socket_sender: writer_sender,
      _socket: socket,
      connected,
      device_event_sender,
    })
  }
}

impl HardwareInternal for UdpHardware {
  fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.device_event_sender.subscribe()
  }

  fn disconnect(&self) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let connected = self.connected.clone();
    async move {
      connected.store(false, Ordering::Relaxed);
      Ok(())
    }
    .boxed()
  }

  fn read_value(
    &self,
    _msg: &HardwareReadCmd,
  ) -> BoxFuture<'static, Result<HardwareReading, ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "UDP does not support read".to_owned(),
    )))
    .boxed()
  }

  fn write_value(
    &self,
    msg: &HardwareWriteCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let sender = self.socket_sender.clone();
    let data = msg.data().clone();
    // TODO Should check endpoint validity
    async move {
      if sender.send(data).await.is_err() {
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
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "UDP does not support subscribe".to_owned(),
    )))
    .boxed()
  }

  fn unsubscribe(
    &self,
    _msg: &HardwareUnsubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "UDP does not support unsubscribe".to_owned(),
    )))
    .boxed()
  }
}

impl Drop for UdpHardware {
  fn drop(&mut self) {
  }
}
