// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2025 Nonpolynomial Labs LLC., Milibyte LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server::device::hardware::GenericHardwareSpecializer;
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
use buttplug_server_device_config::{Endpoint, ProtocolCommunicationSpecifier, UdpSpecifier};
use futures::future;
use futures::{future::BoxFuture, FutureExt};
use std::{
  fmt::{self, Debug},
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
};
use tokio::sync::{broadcast, mpsc::{channel, Receiver, Sender}};
use tokio::net::UdpSocket;

pub struct UdpHardwareConnector {
  specifier: UdpSpecifier,
}

impl UdpHardwareConnector {
  pub fn new(specifier: UdpSpecifier) -> Self {
    Self {
      specifier: specifier.clone(),
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
    ProtocolCommunicationSpecifier::Udp(self.specifier.clone())
  }

  async fn connect(&mut self) -> Result<Box<dyn HardwareSpecializer>, ButtplugDeviceError> {
    let address = self.specifier.address().clone();
    let port = *self.specifier.port();
    let socket = Arc::new(UdpSocket::bind("0.0.0.0:0")
      .await
      .map_err(|e| {
        ButtplugDeviceError::DeviceSpecificError(
          HardwareSpecificError::HardwareSpecificError("UDP-bind".to_owned(), e.to_string()).to_string())
      })?);
    socket.connect(format!("{}:{}", address.clone(), port))
      .await
      .map_err(|e| {
        ButtplugDeviceError::DeviceSpecificError(
          HardwareSpecificError::HardwareSpecificError("UDP-connect".to_owned(), e.to_string()).to_string(),
        )
    })?;
    let hardware_internal = UdpHardware::new(
      socket,
      address,
      port,
    );
    let hardware = Hardware::new(
      &format!("UDP ({})", self.specifier.to_string()).to_owned(),
      &self.specifier.to_string(),
      &[Endpoint::Rx, Endpoint::Tx],
      &None,
      false,
      Box::new(hardware_internal),
    );
    Ok(Box::new(GenericHardwareSpecializer::new(hardware)))
  }
}

async fn udp_write_thread(socket: Arc<UdpSocket>, receiver: Receiver<Vec<u8>>) {
  let mut recv = receiver;
  // Instead of waiting on a token here, we'll expect that we'll break on our
  // channel going away.
  while let Some(v) = recv.recv().await {
    if let Err(err) = socket.send(&v).await {
      warn!("Cannot write data to udp port, exiting thread: {}", err);
      return;
    }
  }
}

pub struct UdpHardware {
  address: Arc<String>,
  port: Arc<u16>,
  socket_sender: Sender<Vec<u8>>,
  connected: Arc<AtomicBool>,
  device_event_sender: broadcast::Sender<HardwareEvent>,
}

impl UdpHardware {
  pub fn name(&self) -> String {
    format!("{}:{}", self.address, self.port)
  }

  pub fn new(socket: Arc<UdpSocket>, address: String, port: u16) -> Self {
    let (outgoing_sender, outgoing_receiver) = channel(256);
    let (device_event_sender, _) = broadcast::channel(256);
    // If we've gotten this far, we can expect we have a socket definition.
    let connected = Arc::new(AtomicBool::new(true));
    let write_socket = socket.clone();
    tokio::spawn(async move {
      udp_write_thread(write_socket, outgoing_receiver).await;
    });

    Self {
      address: Arc::new(address.to_owned()),
      port: Arc::new(port),
      socket_sender: outgoing_sender,
      connected,
      device_event_sender,
    }
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
      if let Err(e) = sender.send(data)
        .await
        .map_err(|e| {
          ButtplugDeviceError::DeviceSpecificError(
            HardwareSpecificError::HardwareSpecificError("UDP send-to-thread".to_owned(), e.to_string()).to_string(),
        )
        })
      {
        warn!("UDP write value: {}", e.to_string());
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
