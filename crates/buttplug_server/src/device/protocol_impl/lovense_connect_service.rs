// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use async_trait::async_trait;
use buttplug_core::{
  errors::ButtplugDeviceError,
  message::{InputReadingV4, OutputType},
};
use buttplug_server_device_config::{
  Endpoint,
  ProtocolCommunicationSpecifier,
  UserDeviceIdentifier,
};
use futures::future::{BoxFuture, FutureExt};
use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};
use uuid::{Uuid, uuid};

use crate::device::{
  hardware::{Hardware, HardwareReadCmd, HardwareWriteCmd},
  protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer},
};
use buttplug_server_device_config::ServerDeviceDefinition;

const LOVENSE_CONNECT_UUID: Uuid = uuid!("590bfbbf-c3b7-41ae-9679-485b190ffb87");

pub mod setup {
  use crate::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};
  #[derive(Default)]
  pub struct LovenseConnectIdentifierFactory {}

  impl ProtocolIdentifierFactory for LovenseConnectIdentifierFactory {
    fn identifier(&self) -> &str {
      "lovense-connect-service"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::LovenseConnectIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct LovenseConnectIdentifier {}

#[async_trait]
impl ProtocolIdentifier for LovenseConnectIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
    _: ProtocolCommunicationSpecifier,
  ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    Ok((
      UserDeviceIdentifier::new(
        hardware.address(),
        "lovense-connect-service",
        &Some(hardware.name().to_owned()),
      ),
      Box::new(LovenseConnectServiceInitializer::default()),
    ))
  }
}

#[derive(Default)]
pub struct LovenseConnectServiceInitializer {}

#[async_trait]
impl ProtocolInitializer for LovenseConnectServiceInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    device_definition: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let mut protocol = LovenseConnectService::new(hardware.address());

    protocol.vibrator_count = device_definition
      .features()
      .values()
      .filter(|x| x.contains_output(OutputType::Vibrate))
      .count();
    protocol.thusting_count = device_definition
      .features()
      .values()
      .filter(|x| x.contains_output(OutputType::Oscillate))
      .count();

    // The Ridge and Gravity both oscillate, but the Ridge only oscillates but takes
    // the vibrate command... The Gravity has a vibe as well, and uses a Thrusting
    // command for that oscillator.
    if protocol.vibrator_count == 0 && protocol.thusting_count != 0 {
      protocol.vibrator_count = protocol.thusting_count;
      protocol.thusting_count = 0;
    }

    if hardware.name() == "Solace" {
      // Just hardcoding this weird exception until we can control depth
      let lovense_cmd = format!("Depth?v={}&t={}", 3, hardware.address())
        .as_bytes()
        .to_vec();

      hardware
        .write_value(&HardwareWriteCmd::new(
          &[LOVENSE_CONNECT_UUID],
          Endpoint::Tx,
          lovense_cmd,
          false,
        ))
        .await?;

      protocol.vibrator_count = 0;
      protocol.thusting_count = 1;
    }

    Ok(Arc::new(protocol))
  }
}

pub struct LovenseConnectService {
  address: String,
  rotation_clockwise: Arc<AtomicBool>,
  vibrator_count: usize,
  thusting_count: usize,
}

impl LovenseConnectService {
  pub fn new(address: &str) -> Self {
    Self {
      address: address.to_owned(),
      // Convention: positive speed is the power-on direction, since Lovense can't report it.
      rotation_clockwise: Arc::new(AtomicBool::new(true)),
      vibrator_count: 0,
      thusting_count: 0,
    }
  }
}

impl ProtocolHandler for LovenseConnectService {
  fn handle_output_cmd(
    &self,
    cmd: &crate::message::checked_output_cmd::CheckedOutputCmdV4,
  ) -> Result<Vec<crate::device::hardware::HardwareCommand>, ButtplugDeviceError> {
    let mut hardware_cmds = vec![];

    // We do all of our validity checking during message conversion to checked, so we should be able to skip validity checking here.
    if cmd.output_command().as_output_type() == OutputType::Vibrate {
      // Sure do hope we're keeping our vibrator indexes aligned with what lovense expects!
      //
      // God I can't wait to fucking kill this stupid protocol.
      let lovense_cmd = format!(
        "Vibrate{}?v={}&t={}",
        cmd.feature_index() + 1,
        cmd.output_command().value(),
        self.address
      )
      .as_bytes()
      .to_vec();
      hardware_cmds.push(
        HardwareWriteCmd::new(&[LOVENSE_CONNECT_UUID], Endpoint::Tx, lovense_cmd, false).into(),
      );
      Ok(hardware_cmds)
    } else if self.thusting_count != 0
      && cmd.output_command().as_output_type() == OutputType::Oscillate
    {
      let lovense_cmd = format!(
        "Thrusting?v={}&t={}",
        cmd.output_command().value(),
        self.address
      )
      .as_bytes()
      .to_vec();
      hardware_cmds.push(
        HardwareWriteCmd::new(&[LOVENSE_CONNECT_UUID], Endpoint::Tx, lovense_cmd, false).into(),
      );
      Ok(hardware_cmds)
    } else if cmd.output_command().as_output_type() == OutputType::Oscillate {
      // Only the max has a constriction system, and there's only one, so just parse the first command.
      /* ~ Sutekh
       * - Implemented constriction.
       * - Kept things consistent with the lovense handle_scalar_cmd() method.
       * - Using AirAuto method.
       * - Changed step count in device config file to 3.
       */
      let lovense_cmd = format!(
        "AirAuto?v={}&t={}",
        cmd.output_command().value(),
        self.address
      )
      .as_bytes()
      .to_vec();

      hardware_cmds.push(
        HardwareWriteCmd::new(&[LOVENSE_CONNECT_UUID], Endpoint::Tx, lovense_cmd, false).into(),
      );
      Ok(hardware_cmds)
    } else {
      Ok(hardware_cmds)
    }
  }

  fn handle_output_rotate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    speed: i32,
  ) -> Result<Vec<crate::device::hardware::HardwareCommand>, ButtplugDeviceError> {
    let mut hardware_cmds = vec![];
    if speed != 0 {
      let clockwise = speed > 0;
      if self.rotation_clockwise.swap(clockwise, Ordering::Relaxed) != clockwise {
        let lovense_cmd = format!("RotateChange?t={}", self.address)
          .as_bytes()
          .to_vec();
        // RotateChange? is a toggle, so a unique id keeps device task batching from deduping it.
        hardware_cmds
          .push(HardwareWriteCmd::new(&[Uuid::new_v4()], Endpoint::Tx, lovense_cmd, false).into());
      }
    }
    let lovense_cmd = format!("Rotate?v={}&t={}", speed.unsigned_abs(), self.address)
      .as_bytes()
      .to_vec();
    hardware_cmds.push(
      HardwareWriteCmd::new(&[LOVENSE_CONNECT_UUID], Endpoint::Tx, lovense_cmd, false).into(),
    );
    Ok(hardware_cmds)
  }

  fn handle_input_read_cmd(
    &self,
    device_index: u32,
    device: Arc<Hardware>,
    feature_index: u32,
    _feature_id: Uuid,
    _sensor_type: buttplug_core::message::InputType,
  ) -> BoxFuture<'_, Result<buttplug_core::message::InputReadingV4, ButtplugDeviceError>> {
    async move {
      // This is a dummy read. We just store the battery level in the device
      // implementation and it's the only thing read will return.
      let reading = device
        .read_value(&HardwareReadCmd::new(
          LOVENSE_CONNECT_UUID,
          Endpoint::Rx,
          0,
          0,
        ))
        .await?;
      debug!("Battery level: {}", reading.data()[0]);
      Ok(InputReadingV4::new(
        device_index,
        feature_index,
        buttplug_core::message::InputTypeReading::Battery(reading.data()[0].into()),
      ))
    }
    .boxed()
  }
}

#[cfg(test)]
mod tests {
  use super::LovenseConnectService;
  use crate::device::{hardware::HardwareCommand, protocol::ProtocolHandler};
  use uuid::Uuid;

  fn rotate_commands(protocol: &LovenseConnectService, speed: i32) -> Vec<Vec<u8>> {
    protocol
      .handle_output_rotate_cmd(0, Uuid::nil(), speed)
      .expect("command should be valid")
      .into_iter()
      .map(|cmd| match cmd {
        HardwareCommand::Write(write_cmd) => write_cmd.data().clone(),
        _ => panic!("Expected a write command"),
      })
      .collect()
  }

  fn contains_rotate_change(commands: &[Vec<u8>]) -> bool {
    commands
      .iter()
      .any(|data| data.starts_with(b"RotateChange?"))
  }

  fn rotate_magnitude(commands: &[Vec<u8>]) -> String {
    let data = commands
      .iter()
      .find(|data| data.starts_with(b"Rotate?"))
      .expect("Expected a Rotate command");
    String::from_utf8(data.clone()).expect("Command should be valid utf8")
  }

  #[test]
  fn initial_positive_speed_does_not_toggle() {
    let protocol = LovenseConnectService::new("");
    let commands = rotate_commands(&protocol, 20);
    assert!(!contains_rotate_change(&commands));
  }

  #[test]
  fn repeated_negative_speeds_toggle_once() {
    let protocol = LovenseConnectService::new("");
    assert!(contains_rotate_change(&rotate_commands(&protocol, -20)));
    assert!(!contains_rotate_change(&rotate_commands(&protocol, -10)));
    assert!(!contains_rotate_change(&rotate_commands(&protocol, -5)));
  }

  #[test]
  fn negative_then_positive_toggles_back() {
    let protocol = LovenseConnectService::new("");
    assert!(contains_rotate_change(&rotate_commands(&protocol, -20)));
    assert!(contains_rotate_change(&rotate_commands(&protocol, 20)));
  }

  #[test]
  fn zero_speed_does_not_toggle() {
    let protocol = LovenseConnectService::new("");
    assert!(!contains_rotate_change(&rotate_commands(&protocol, 0)));
    assert!(contains_rotate_change(&rotate_commands(&protocol, -20)));
    assert!(!contains_rotate_change(&rotate_commands(&protocol, -10)));
  }

  #[test]
  fn rotate_command_sends_magnitude_not_signed_speed() {
    let protocol = LovenseConnectService::new("addr");
    let commands = rotate_commands(&protocol, -20);
    let rotate_url = rotate_magnitude(&commands);
    assert_eq!(rotate_url, "Rotate?v=20&t=addr");
  }

  #[test]
  fn rotate_change_includes_toy_address() {
    let protocol = LovenseConnectService::new("addr");
    let commands = rotate_commands(&protocol, -20);
    let rotate_change = commands
      .iter()
      .find(|data| data.starts_with(b"RotateChange?"))
      .expect("Expected a RotateChange command");
    assert_eq!(
      String::from_utf8(rotate_change.clone()).expect("Command should be valid utf8"),
      "RotateChange?t=addr"
    );
  }

  #[test]
  fn rotate_change_precedes_rotate_speed_write() {
    let protocol = LovenseConnectService::new("addr");
    let commands = rotate_commands(&protocol, -20);
    assert_eq!(commands.len(), 2);
    assert!(commands[0].starts_with(b"RotateChange?"));
    assert!(commands[1].starts_with(b"Rotate?"));
  }

  #[test]
  fn toggle_writes_do_not_share_command_ids() {
    let protocol = LovenseConnectService::new("addr");
    let first_call = protocol
      .handle_output_rotate_cmd(0, Uuid::nil(), -20)
      .expect("command should be valid");
    let second_call = protocol
      .handle_output_rotate_cmd(0, Uuid::nil(), 20)
      .expect("command should be valid");

    let is_rotate_change = |cmd: &&HardwareCommand| matches!(cmd, HardwareCommand::Write(w) if w.data().starts_with(b"RotateChange?"));
    let is_rotate_speed = |cmd: &&HardwareCommand| matches!(cmd, HardwareCommand::Write(w) if w.data().starts_with(b"Rotate?"));

    let first_toggle = first_call
      .iter()
      .find(is_rotate_change)
      .expect("Expected a RotateChange command in the first call");
    let second_toggle = second_call
      .iter()
      .find(is_rotate_change)
      .expect("Expected a RotateChange command in the second call");
    let speed_write = second_call
      .iter()
      .find(is_rotate_speed)
      .expect("Expected a Rotate speed command");

    assert!(!first_toggle.overlaps(second_toggle));
    assert!(!first_toggle.overlaps(speed_write));
    assert!(!second_toggle.overlaps(speed_write));
  }
}
