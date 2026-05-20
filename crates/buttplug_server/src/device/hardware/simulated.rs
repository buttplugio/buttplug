// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::hardware::{
  Hardware, HardwareConnector, HardwareEvent, HardwareInternal, HardwareReadCmd, HardwareReading,
  HardwareSpecializer, HardwareSubscribeCmd, HardwareUnsubscribeCmd, HardwareWriteCmd,
};
use crate::device::hardware::communication::{
  HardwareCommunicationManager, HardwareCommunicationManagerBuilder,
  HardwareCommunicationManagerEvent,
};
use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_core::ButtplugResultFuture;
use buttplug_server_device_config::{Endpoint, ProtocolCommunicationSpecifier, SimulatedSpecifier};
use futures::future::{self, BoxFuture, FutureExt};
use std::fmt::{self, Debug};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::mpsc::Sender;
use tracing::error;

pub struct SimulatedHardwareInternal {
  address: String,
  event_sender: broadcast::Sender<HardwareEvent>,
}

impl SimulatedHardwareInternal {
  pub fn new(address: &str) -> Self {
    let (event_sender, _) = broadcast::channel(256);
    Self {
      address: address.to_owned(),
      event_sender,
    }
  }
}

impl HardwareInternal for SimulatedHardwareInternal {
  fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.event_sender.subscribe()
  }

  fn disconnect(&self) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let sender = self.event_sender.clone();
    let address = self.address.clone();
    async move {
      let _ = sender.send(HardwareEvent::Disconnected(address));
      Ok(())
    }
    .boxed()
  }

  fn read_value(
    &self,
    msg: &HardwareReadCmd,
  ) -> BoxFuture<'static, Result<HardwareReading, ButtplugDeviceError>> {
    let endpoint = msg.endpoint();
    future::ready(Ok(HardwareReading::new(endpoint, &[]))).boxed()
  }

  fn write_value(
    &self,
    _msg: &HardwareWriteCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Ok(())).boxed()
  }

  fn subscribe(
    &self,
    _msg: &HardwareSubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Ok(())).boxed()
  }

  fn unsubscribe(
    &self,
    _msg: &HardwareUnsubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Ok(())).boxed()
  }
}

pub struct SimulatedHardwareConnector {
  specifier: ProtocolCommunicationSpecifier,
  hardware: Option<SimulatedHardwareInternal>,
}

impl SimulatedHardwareConnector {
  pub fn new(specifier: ProtocolCommunicationSpecifier, hardware: SimulatedHardwareInternal) -> Self {
    Self {
      specifier,
      hardware: Some(hardware),
    }
  }
}

impl Debug for SimulatedHardwareConnector {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("SimulatedHardwareConnector")
      .field("specifier", &self.specifier)
      .finish()
  }
}

#[async_trait]
impl HardwareConnector for SimulatedHardwareConnector {
  fn specifier(&self) -> ProtocolCommunicationSpecifier {
    self.specifier.clone()
  }

  async fn connect(&mut self) -> Result<Box<dyn HardwareSpecializer>, ButtplugDeviceError> {
    Ok(Box::new(SimulatedHardwareSpecializer {
      hardware: self.hardware.take(),
    }))
  }
}

pub struct SimulatedHardwareSpecializer {
  hardware: Option<SimulatedHardwareInternal>,
}

#[async_trait]
impl HardwareSpecializer for SimulatedHardwareSpecializer {
  async fn specialize(
    &mut self,
    _specifiers: &[ProtocolCommunicationSpecifier],
  ) -> Result<Hardware, ButtplugDeviceError> {
    let device = self
      .hardware
      .take()
      .ok_or(ButtplugDeviceError::DeviceConnectionError(
        "Simulated hardware already taken".to_owned(),
      ))?;
    let address = device.address.clone();
    let endpoints = vec![Endpoint::Tx];
    Ok(Hardware::new(
      "Simulated Device",
      &address,
      &endpoints,
      &None,
      false,
      Box::new(device),
    ))
  }
}

#[derive(Clone, Debug)]
pub struct SimulatedDeviceEntry {
  pub identifier: String,
  pub display_name: Option<String>,
  pub address: String,
}

pub struct SimulatedHardwareCommunicationManagerBuilder {
  devices: Vec<SimulatedDeviceEntry>,
}

impl SimulatedHardwareCommunicationManagerBuilder {
  pub fn new(devices: Vec<SimulatedDeviceEntry>) -> Self {
    Self { devices }
  }
}

impl HardwareCommunicationManagerBuilder for SimulatedHardwareCommunicationManagerBuilder {
  fn finish(
    &mut self,
    sender: Sender<HardwareCommunicationManagerEvent>,
  ) -> Box<dyn HardwareCommunicationManager> {
    let devices = std::mem::take(&mut self.devices);
    Box::new(SimulatedHardwareCommunicationManager::new(sender, devices))
  }
}

pub struct SimulatedHardwareCommunicationManager {
  device_sender: Sender<HardwareCommunicationManagerEvent>,
  devices: Vec<SimulatedDeviceEntry>,
  is_scanning: Arc<AtomicBool>,
}

impl SimulatedHardwareCommunicationManager {
  fn new(
    device_sender: Sender<HardwareCommunicationManagerEvent>,
    devices: Vec<SimulatedDeviceEntry>,
  ) -> Self {
    Self {
      device_sender,
      devices,
      is_scanning: Arc::new(AtomicBool::new(false)),
    }
  }
}

impl HardwareCommunicationManager for SimulatedHardwareCommunicationManager {
  fn name(&self) -> &'static str {
    "SimulatedHardwareCommunicationManager"
  }

  fn start_scanning(&mut self) -> ButtplugResultFuture {
    let mut events = vec![];

    for device in &self.devices {
      let name = device
        .display_name
        .as_deref()
        .unwrap_or(&device.identifier);
      let specifier = ProtocolCommunicationSpecifier::Simulated(
        SimulatedSpecifier::new(&device.identifier),
      );
      let hardware = SimulatedHardwareInternal::new(&device.address);
      let connector = SimulatedHardwareConnector::new(specifier, hardware);

      events.push(HardwareCommunicationManagerEvent::DeviceFound {
        name: name.to_owned(),
        address: device.address.clone(),
        creator: Box::new(connector),
      });
    }

    let device_sender = self.device_sender.clone();
    let is_scanning = self.is_scanning.clone();
    async move {
      is_scanning.store(true, Ordering::Relaxed);
      for event in events {
        if device_sender.send(event).await.is_err() {
          error!("Simulated device channel no longer open.");
        }
      }
      is_scanning.store(false, Ordering::Relaxed);
      if device_sender
        .send(HardwareCommunicationManagerEvent::ScanningFinished)
        .await
        .is_err()
      {
        error!("Error sending scanning finished for simulated devices.");
      }
      Ok(())
    }
    .boxed()
  }

  fn stop_scanning(&mut self) -> ButtplugResultFuture {
    future::ready(Ok(())).boxed()
  }

  fn can_scan(&self) -> bool {
    !self.devices.is_empty()
  }

  fn scanning_status(&self) -> bool {
    self.is_scanning.load(Ordering::Relaxed)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_simulated_hardware_internal_new() {
    let hardware = SimulatedHardwareInternal::new("test-address");
    assert_eq!(hardware.address, "test-address");
  }

  #[test]
  fn test_simulated_hardware_internal_event_stream() {
    let hardware = SimulatedHardwareInternal::new("test-address");
    let _receiver = hardware.event_stream();
    // Just verify it doesn't panic
  }

  #[tokio::test]
  async fn test_simulated_hardware_internal_write_value() {
    let hardware = SimulatedHardwareInternal::new("test-address");
    let uuid = uuid::Uuid::new_v4();
    let cmd = HardwareWriteCmd::new(&[uuid], Endpoint::Tx, vec![1, 2, 3], false);
    let result = hardware.write_value(&cmd).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_simulated_hardware_internal_read_value() {
    let hardware = SimulatedHardwareInternal::new("test-address");
    let cmd = HardwareReadCmd::new(uuid::Uuid::new_v4(), Endpoint::Tx, 10, 1000);
    let result = hardware.read_value(&cmd).await;
    assert!(result.is_ok());
    let reading = result.unwrap();
    assert_eq!(reading.data().len(), 0);
    assert_eq!(*reading.endpoint(), Endpoint::Tx);
  }

  #[tokio::test]
  async fn test_simulated_hardware_internal_subscribe() {
    let hardware = SimulatedHardwareInternal::new("test-address");
    let cmd = HardwareSubscribeCmd::new(uuid::Uuid::new_v4(), Endpoint::Tx);
    let result = hardware.subscribe(&cmd).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_simulated_hardware_internal_unsubscribe() {
    let hardware = SimulatedHardwareInternal::new("test-address");
    let cmd = HardwareUnsubscribeCmd::new(uuid::Uuid::new_v4(), Endpoint::Tx);
    let result = hardware.unsubscribe(&cmd).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_simulated_hardware_internal_disconnect() {
    let hardware = SimulatedHardwareInternal::new("test-address");
    let mut receiver = hardware.event_stream();

    let disconnect_result = hardware.disconnect().await;
    assert!(disconnect_result.is_ok());

    // Check that disconnect event was sent
    let event = receiver.recv().await;
    assert!(event.is_ok());
    match event.unwrap() {
      HardwareEvent::Disconnected(addr) => assert_eq!(addr, "test-address"),
      _ => panic!("Expected Disconnected event"),
    }
  }

  #[test]
  fn test_simulated_hardware_connector_specifier() {
    let specifier = ProtocolCommunicationSpecifier::Simulated(SimulatedSpecifier::new("test-id"));
    let hardware = SimulatedHardwareInternal::new("test-address");
    let connector = SimulatedHardwareConnector::new(specifier.clone(), hardware);

    assert_eq!(connector.specifier(), specifier);
  }

  #[tokio::test]
  async fn test_simulated_hardware_connector_connect() {
    let specifier = ProtocolCommunicationSpecifier::Simulated(SimulatedSpecifier::new("test-id"));
    let hardware = SimulatedHardwareInternal::new("test-address");
    let mut connector = SimulatedHardwareConnector::new(specifier, hardware);

    let result = connector.connect().await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_simulated_hardware_specializer_specialize() {
    let specifier = ProtocolCommunicationSpecifier::Simulated(SimulatedSpecifier::new("test-id"));
    let hardware = SimulatedHardwareInternal::new("test-address");
    let mut connector = SimulatedHardwareConnector::new(specifier.clone(), hardware);

    let mut specializer = connector.connect().await.unwrap();
    let result = specializer.specialize(&[specifier]).await;

    assert!(result.is_ok());
    let hw = result.unwrap();
    assert_eq!(hw.address(), "test-address");
    assert_eq!(hw.endpoints().len(), 1);
    assert_eq!(hw.endpoints()[0], Endpoint::Tx);
  }

  #[test]
  fn test_simulated_hardware_communication_manager_builder() {
    let devices = vec![
      SimulatedDeviceEntry {
        identifier: "device1".to_string(),
        display_name: Some("Test Device 1".to_string()),
        address: "addr1".to_string(),
      },
    ];
    let builder = SimulatedHardwareCommunicationManagerBuilder::new(devices);
    assert_eq!(builder.devices.len(), 1);
  }

  #[tokio::test]
  async fn test_simulated_hardware_communication_manager_can_scan_with_devices() {
    let devices = vec![
      SimulatedDeviceEntry {
        identifier: "device1".to_string(),
        display_name: Some("Test Device 1".to_string()),
        address: "addr1".to_string(),
      },
    ];
    let (tx, _rx) = tokio::sync::mpsc::channel(10);
    let manager = SimulatedHardwareCommunicationManager::new(tx, devices);

    assert!(manager.can_scan());
  }

  #[tokio::test]
  async fn test_simulated_hardware_communication_manager_can_scan_without_devices() {
    let devices = vec![];
    let (tx, _rx) = tokio::sync::mpsc::channel(10);
    let manager = SimulatedHardwareCommunicationManager::new(tx, devices);

    assert!(!manager.can_scan());
  }

  #[tokio::test]
  async fn test_simulated_hardware_communication_manager_start_scanning() {
    let devices = vec![
      SimulatedDeviceEntry {
        identifier: "device1".to_string(),
        display_name: Some("Test Device 1".to_string()),
        address: "addr1".to_string(),
      },
      SimulatedDeviceEntry {
        identifier: "device2".to_string(),
        display_name: None,
        address: "addr2".to_string(),
      },
    ];
    let (tx, mut rx) = tokio::sync::mpsc::channel(10);
    let mut manager = SimulatedHardwareCommunicationManager::new(tx, devices);

    let scan_result = manager.start_scanning().await;
    assert!(scan_result.is_ok());

    // Should receive DeviceFound events
    let event1 = rx.recv().await;
    assert!(event1.is_some());
    match event1.unwrap() {
      HardwareCommunicationManagerEvent::DeviceFound { name, address, .. } => {
        assert_eq!(name, "Test Device 1");
        assert_eq!(address, "addr1");
      }
      _ => panic!("Expected DeviceFound event"),
    }

    let event2 = rx.recv().await;
    assert!(event2.is_some());
    match event2.unwrap() {
      HardwareCommunicationManagerEvent::DeviceFound { name, address, .. } => {
        assert_eq!(name, "device2");
        assert_eq!(address, "addr2");
      }
      _ => panic!("Expected DeviceFound event"),
    }

    // Should receive ScanningFinished event
    let event3 = rx.recv().await;
    assert!(event3.is_some());
    match event3.unwrap() {
      HardwareCommunicationManagerEvent::ScanningFinished => {}
      _ => panic!("Expected ScanningFinished event"),
    }
  }

  #[tokio::test]
  async fn test_simulated_hardware_communication_manager_stop_scanning() {
    let devices = vec![];
    let (tx, _rx) = tokio::sync::mpsc::channel(10);
    let mut manager = SimulatedHardwareCommunicationManager::new(tx, devices);

    let result = manager.stop_scanning().await;
    assert!(result.is_ok());
  }

  #[test]
  fn test_simulated_hardware_communication_manager_name() {
    let devices = vec![];
    let (tx, _rx) = tokio::sync::mpsc::channel(10);
    let manager = SimulatedHardwareCommunicationManager::new(tx, devices);

    assert_eq!(manager.name(), "SimulatedHardwareCommunicationManager");
  }
}
