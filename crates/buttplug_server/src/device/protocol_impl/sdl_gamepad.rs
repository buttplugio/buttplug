// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_core::message::{InputReadingV4, InputTypeReading, InputValue};
use buttplug_server_device_config::{Endpoint, ProtocolCommunicationSpecifier};
use buttplug_server_device_config::{
  SdlGamepadLayout,
  ServerDeviceDefinition,
  UserDeviceIdentifier,
};
use byteorder::{LittleEndian, WriteBytesExt};
use futures::{FutureExt, future::BoxFuture};
use std::sync::{Arc, Mutex};

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareReadCmd, HardwareWriteCmd},
  protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolIdentifierFactory, ProtocolInitializer},
};

pub mod setup {
  use super::*;

  #[derive(Default)]
  pub struct SdlGamepadIdentifierFactory {}

  impl ProtocolIdentifierFactory for SdlGamepadIdentifierFactory {
    fn identifier(&self) -> &str {
      "sdl-gamepad"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(SdlGamepadIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct SdlGamepadIdentifier {}

#[async_trait]
impl ProtocolIdentifier for SdlGamepadIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
    _: ProtocolCommunicationSpecifier,
  ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    let identifier = UserDeviceIdentifier::new(
      hardware.address(),
      "sdl-gamepad",
      &Some(hardware.name().to_owned()),
    );
    Ok((identifier, Box::new(SdlGamepadInitializer::default())))
  }
}

#[derive(Default)]
pub struct SdlGamepadInitializer {}

#[async_trait]
impl ProtocolInitializer for SdlGamepadInitializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    device_definition: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let layout =
      SdlGamepadLayout::from_protocol_variant(device_definition.protocol_variant().as_deref());
    Ok(Arc::new(SdlGamepad::new(layout)))
  }
}

/// SDL3 gamepad rumble protocol.
///
/// Every vibrate command carries the complete logical state. The handler keeps
/// the last-set speed for all four logical slots and packs all four u16 values
/// (little-endian) into every write packet. The internal packet is 8 bytes:
/// [low-frequency main, high-frequency main, left trigger, right trigger].
///
/// Visible feature indexes are mapped by the final device definition's layout:
/// MainOnly maps 0/1 to slots 0/1, TriggersOnly maps 0/1 to slots 2/3, and
/// MainAndTriggers maps 0-3 to slots 0-3. The layout comes from the protocol
/// variant, never from the feature count. Disabled features are filtered before
/// the handler sees them and must not be reinterpreted as different hardware
/// channels.
pub struct SdlGamepad {
  layout: SdlGamepadLayout,
  slots: Mutex<[u16; 4]>,
}

impl SdlGamepad {
  pub fn new(layout: SdlGamepadLayout) -> Self {
    Self {
      layout,
      slots: Mutex::new([0; 4]),
    }
  }
}

impl Default for SdlGamepad {
  fn default() -> Self {
    Self::new(SdlGamepadLayout::MainOnly)
  }
}

impl ProtocolHandler for SdlGamepad {
  fn handle_battery_level_cmd(
    &self,
    device_index: u32,
    device: Arc<Hardware>,
    feature_index: u32,
    feature_id: uuid::Uuid,
  ) -> BoxFuture<'_, Result<InputReadingV4, ButtplugDeviceError>> {
    debug!("Trying to get SDL gamepad battery reading.");
    let msg = HardwareReadCmd::new(feature_id, Endpoint::Rx, 1, 0);
    let fut = device.read_value(&msg);
    async move {
      let hw_msg = fut.await?;
      let battery_level = hw_msg.data()[0] as i32;
      let battery_reading = InputReadingV4::new(
        device_index,
        feature_index,
        InputTypeReading::Battery(InputValue::new(battery_level as u8)),
      );
      debug!("Got SDL gamepad battery reading: {}", battery_level);
      Ok(battery_reading)
    }
    .boxed()
  }

  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if feature_index as usize >= self.layout.channel_count() {
      return Err(ButtplugDeviceError::ProtocolSpecificError(
        "SdlGamepad".to_owned(),
        format!(
          "SDL gamepad only has {} vibrate features, got index {feature_index}",
          self.layout.channel_count()
        ),
      ));
    }

    let mut slots = self.slots.lock().unwrap();
    let slot = self.layout.logical_slots()[feature_index as usize] as usize;
    slots[slot] = speed as u16;
    let mut cmd = vec![];
    for speed in slots.iter() {
      if cmd.write_u16::<LittleEndian>(*speed).is_err() {
        return Err(ButtplugDeviceError::ProtocolSpecificError(
          "SdlGamepad".to_owned(),
          "Cannot convert SDL gamepad value for processing".to_owned(),
        ));
      }
    }

    Ok(vec![
      HardwareWriteCmd::new(&[feature_id], Endpoint::Tx, cmd, false).into(),
    ])
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::device::hardware::{
    HardwareEvent,
    HardwareInternal,
    HardwareReading,
    HardwareSubscribeCmd,
    HardwareUnsubscribeCmd,
  };
  use buttplug_core::message::ButtplugDeviceMessage;
  use futures::future;
  use tokio::sync::broadcast;

  struct BatteryHardware;

  impl HardwareInternal for BatteryHardware {
    fn disconnect(&self) -> futures::future::BoxFuture<'static, Result<(), ButtplugDeviceError>> {
      future::ready(Ok(())).boxed()
    }

    fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
      broadcast::channel(1).0.subscribe()
    }

    fn read_value(
      &self,
      _msg: &HardwareReadCmd,
    ) -> futures::future::BoxFuture<'static, Result<HardwareReading, ButtplugDeviceError>> {
      future::ready(Ok(HardwareReading::new(Endpoint::Rx, &[77]))).boxed()
    }

    fn write_value(
      &self,
      _msg: &HardwareWriteCmd,
    ) -> futures::future::BoxFuture<'static, Result<(), ButtplugDeviceError>> {
      future::ready(Err(ButtplugDeviceError::UnhandledCommand(
        "write".to_owned(),
      )))
      .boxed()
    }

    fn subscribe(
      &self,
      _msg: &HardwareSubscribeCmd,
    ) -> futures::future::BoxFuture<'static, Result<(), ButtplugDeviceError>> {
      future::ready(Err(ButtplugDeviceError::UnhandledCommand(
        "subscribe".to_owned(),
      )))
      .boxed()
    }

    fn unsubscribe(
      &self,
      _msg: &HardwareUnsubscribeCmd,
    ) -> futures::future::BoxFuture<'static, Result<(), ButtplugDeviceError>> {
      future::ready(Err(ButtplugDeviceError::UnhandledCommand(
        "unsubscribe".to_owned(),
      )))
      .boxed()
    }
  }

  #[tokio::test]
  async fn sdl_protocol_battery_read_wraps_input_reading() {
    let hardware = Arc::new(Hardware::new(
      "SDL Gamepad",
      "sdl-gamepad-1",
      &[Endpoint::Tx, Endpoint::Rx],
      &None,
      false,
      Box::new(BatteryHardware),
    ));
    let reading = SdlGamepad::new(SdlGamepadLayout::MainOnly)
      .handle_battery_level_cmd(3, hardware, 2, uuid::Uuid::new_v4())
      .await
      .expect("battery protocol read should succeed");
    assert_eq!(reading.device_index(), 3);
    assert_eq!(reading.feature_index(), 2);
    assert_eq!(
      reading.reading(),
      InputTypeReading::Battery(InputValue::new(77))
    );
  }

  fn vibrate(handler: &SdlGamepad, feature_index: u32, speed: u32) -> Vec<u8> {
    let cmds = handler
      .handle_output_vibrate_cmd(feature_index, uuid::Uuid::new_v4(), speed)
      .expect("vibrate command should build");
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
      HardwareCommand::Write(write_cmd) => {
        assert_eq!(write_cmd.endpoint(), Endpoint::Tx);
        write_cmd.data().clone()
      }
      _ => panic!("expected a write command"),
    }
  }

  fn packet(values: [u16; 4]) -> Vec<u8> {
    values
      .iter()
      .flat_map(|value| value.to_le_bytes())
      .collect()
  }

  #[test]
  fn sdl_protocol_layout_packets() {
    let cases = [
      (
        SdlGamepadLayout::MainOnly,
        &[(0, 0x1234u16), (1, 0x5678u16)][..],
      ),
      (
        SdlGamepadLayout::TriggersOnly,
        &[(0, 0x1234u16), (1, 0x5678u16)][..],
      ),
      (
        SdlGamepadLayout::MainAndTriggers,
        &[
          (0, 0x1234u16),
          (1, 0x5678u16),
          (2, 0x9abcu16),
          (3, 0xdef0u16),
        ][..],
      ),
    ];

    for (layout, writes) in cases {
      let handler = SdlGamepad::new(layout);
      for &(index, speed) in writes {
        let mut expected = [0; 4];
        for &(previous_index, previous_speed) in writes {
          if previous_index <= index {
            expected[layout.logical_slots()[previous_index as usize] as usize] = previous_speed;
          }
          if previous_index == index {
            break;
          }
        }
        assert_eq!(vibrate(&handler, index, speed as u32), packet(expected));
      }
      assert!(
        handler
          .handle_output_vibrate_cmd(layout.channel_count() as u32, uuid::Uuid::new_v4(), 100,)
          .is_err()
      );
    }
  }

  #[test]
  fn sdl_protocol_rejects_out_of_range_feature_per_layout() {
    for layout in [
      SdlGamepadLayout::MainOnly,
      SdlGamepadLayout::TriggersOnly,
      SdlGamepadLayout::MainAndTriggers,
    ] {
      let error = SdlGamepad::new(layout)
        .handle_output_vibrate_cmd(layout.channel_count() as u32, uuid::Uuid::new_v4(), 100)
        .expect_err("out-of-range feature should be rejected");
      assert!(
        error
          .to_string()
          .contains(&layout.channel_count().to_string())
      );
    }
  }

  #[test]
  fn sdl_protocol_stop_zeroes_only_selected_slot() {
    let handler = SdlGamepad::new(SdlGamepadLayout::MainAndTriggers);
    vibrate(&handler, 0, 0x1234);
    vibrate(&handler, 2, 0x5678);
    assert_eq!(vibrate(&handler, 0, 0), packet([0, 0, 0x5678, 0]));
  }
}
