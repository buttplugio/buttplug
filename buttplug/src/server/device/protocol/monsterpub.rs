// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::sync::Arc;

use async_trait::async_trait;
use futures_util::{future, FutureExt};
use futures_util::future::BoxFuture;

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{ActuatorType, Endpoint},
  },
  server::device::{
    configuration::ProtocolDeviceAttributes,
    configuration::UserDeviceIdentifier,
    hardware::{Hardware, HardwareCommand, HardwareReadCmd, HardwareWriteCmd},
    protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer},
  },
};
use crate::core::message;
use crate::core::message::{ButtplugDeviceMessage, ButtplugServerMessage, SensorReadCmd, SensorType};
use crate::server::device::configuration::ProtocolCommunicationSpecifier;
use crate::server::device::hardware::{HardwareEvent, HardwareSubscribeCmd, HardwareUnsubscribeCmd};

pub mod setup {
  use crate::server::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};

  #[derive(Default)]
  pub struct MonsterPubIdentifierFactory {}

  impl ProtocolIdentifierFactory for MonsterPubIdentifierFactory {
    fn identifier(&self) -> &str {
      "monsterpub"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::MonsterPubIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct MonsterPubIdentifier {}

#[async_trait]
impl ProtocolIdentifier for MonsterPubIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
    _: ProtocolCommunicationSpecifier,
  ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    let read_resp = hardware
      .read_value(&HardwareReadCmd::new(Endpoint::RxBLEModel, 32, 500))
      .await;
    let ident = match read_resp {
      Ok(data) => std::str::from_utf8(&data.data())
        .map_err(|_| {
          ButtplugDeviceError::ProtocolSpecificError(
            "monsterpub".to_owned(),
            "MonsterPub device name is non-UTF8 string.".to_owned(),
          )
        })?
        .replace("\0", "")
        .to_owned(),
      Err(_) => "Unknown".to_string(),
    };
    return Ok((
      UserDeviceIdentifier::new(hardware.address(), "monsterpub", &Some(ident)),
      Box::new(MonsterPubInitializer::default()),
    ));
  }
}

#[derive(Default)]
pub struct MonsterPubInitializer {}

#[async_trait]
impl ProtocolInitializer for MonsterPubInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    if hardware.endpoints().contains(&Endpoint::Rx) {
      let value = hardware
        .read_value(&HardwareReadCmd::new(Endpoint::Rx, 16, 200))
        .await?;
      let keys = [
        [
          0x32u8, 0x49, 0x50, 0x4f, 0x32, 0x49, 0x50, 0x4f, 0x32, 0x49, 0x50, 0x4f, 0x32, 0x49,
          0x50,
        ],
        [
          0x4c, 0x53, 0x42, 0x42, 0x4c, 0x53, 0x42, 0x42, 0x4c, 0x53, 0x42, 0x42, 0x4c, 0x53, 0x42,
        ],
        [
          0x53, 0x49, 0x53, 0x36, 0x53, 0x49, 0x53, 0x36, 0x53, 0x49, 0x53, 0x36, 0x53, 0x49, 0x53,
        ],
        [
          0x54, 0x41, 0x4c, 0x4b, 0x54, 0x41, 0x4c, 0x4b, 0x54, 0x41, 0x4c, 0x4b, 0x54, 0x41, 0x4c,
        ],
      ];

      let auth = value.data()[1..16]
        .iter()
        .zip(keys[value.data()[0] as usize].iter())
        .map(|(&x1, &x2)| x1 ^ x2)
        .collect();

      trace!(
        "Got {:?} XOR with key {} to get {:?}",
        value.data(),
        value.data()[0],
        auth
      );

      hardware
        .write_value(&HardwareWriteCmd::new(Endpoint::Rx, auth, true))
        .await?;
    }
    Ok(Arc::new(MonsterPub::new(
      if hardware.endpoints().contains(&Endpoint::TxVibrate) {
        Endpoint::TxVibrate
      } else {
        Endpoint::Tx
      },
    )))
  }
}

pub struct MonsterPub {
  tx: Endpoint,
}

impl MonsterPub {
  pub fn new(tx: Endpoint) -> Self { Self { tx } }

  // pressure endpoint is notify-only and subscriptions are gonna be reworked in v4
  fn handle_pressure_read_cmd(&self, device: Arc<Hardware>, message: SensorReadCmd) -> BoxFuture<Result<ButtplugServerMessage, ButtplugDeviceError>> {
    let mut device_notification_receiver = device.event_stream();
    async move {
      device.subscribe(&HardwareSubscribeCmd::new(Endpoint::RxPressure)).await?;
      while let Ok(event) = device_notification_receiver.recv().await {
        return match event {
          HardwareEvent::Notification(_, endpoint, data) => {
            if endpoint != Endpoint::RxPressure { continue; }
            if data.len() < 2 {
              return Err(ButtplugDeviceError::ProtocolSpecificError(
                "monsterpub".to_owned(),
                "MonsterPub device returned unexpected data while getting pressure info.".to_owned(),
              ));
            }
            device.unsubscribe(&HardwareUnsubscribeCmd::new(Endpoint::RxPressure)).await?;

            // value is u32 LE, but real value is in range from 0 to about 1000 (0x4ff) (i was scared to squeeze it harder)
            let pressure_level = [data[0], data[1], 0, 0];
            let pressure_reading = message::SensorReading::new(
              message.device_index(),
              *message.sensor_index(),
              SensorType::Pressure,
              vec![i32::from_le_bytes(pressure_level)],
            );

            Ok(pressure_reading.into())
          }
          HardwareEvent::Disconnected(_) => {
            Err(ButtplugDeviceError::ProtocolSpecificError(
              "monsterpub".to_owned(),
              "MonsterPub device disconnected while getting pressure info.".to_owned(),
            ))
          }
        }
      }
      Err(ButtplugDeviceError::ProtocolSpecificError(
        "monsterpub".to_owned(),
        "MonsterPub device disconnected while getting pressure info.".to_owned(),
      ))
    }
      .boxed()
  }
}

impl ProtocolHandler for MonsterPub {
  fn needs_full_command_set(&self) -> bool {
    true
  }

  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut data = vec![];
    let mut stop = true;
    for (_, cmd) in cmds.iter().enumerate() {
      if let Some((_, speed)) = cmd {
        data.push(*speed as u8);
        if *speed != 0 {
          stop = false;
        }
      }
    }
    let tx = if self.tx == Endpoint::Tx && stop {
      Endpoint::TxMode
    } else {
      self.tx
    };
    Ok(vec![HardwareWriteCmd::new(
      tx,
      data,
      tx == Endpoint::TxMode,
    )
    .into()])
  }

  fn handle_sensor_read_cmd(&self, device: Arc<Hardware>, message: SensorReadCmd) -> BoxFuture<Result<ButtplugServerMessage, ButtplugDeviceError>> {
    match message.sensor_type() {
      SensorType::Battery => self.handle_battery_level_cmd(device, message),
      SensorType::Pressure => self.handle_pressure_read_cmd(device, message),
      _ => future::ready(Err(ButtplugDeviceError::UnhandledCommand(
        "Command not implemented for this protocol: SensorReadCmd".to_string(),
      ))).boxed(),
    }
  }
}
