// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
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
      .iter()
      .filter(|x| {
        if let Some(o) = x.1.output() {
          o.contains(OutputType::Vibrate)
        } else {
          false
        }
      })
      .count();
    protocol.thusting_count = device_definition
      .features()
      .iter()
      .filter(|x| {
        if let Some(o) = x.1.output() {
          o.contains(OutputType::Oscillate)
        } else {
          false
        }
      })
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
          &vec![LOVENSE_CONNECT_UUID],
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

#[derive(Default)]
pub struct LovenseConnectService {
  address: String,
  rotation_direction: Arc<AtomicBool>,
  vibrator_count: usize,
  thusting_count: usize,
}

impl LovenseConnectService {
  pub fn new(address: &str) -> Self {
    Self {
      address: address.to_owned(),
      ..Default::default()
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
        HardwareWriteCmd::new(
          &vec![LOVENSE_CONNECT_UUID],
          Endpoint::Tx,
          lovense_cmd,
          false,
        )
        .into(),
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
        HardwareWriteCmd::new(
          &vec![LOVENSE_CONNECT_UUID],
          Endpoint::Tx,
          lovense_cmd,
          false,
        )
        .into(),
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
        HardwareWriteCmd::new(
          &vec![LOVENSE_CONNECT_UUID],
          Endpoint::Tx,
          lovense_cmd,
          false,
        )
        .into(),
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
    let lovense_cmd = format!("/Rotate?v={}&t={}", speed, self.address)
      .as_bytes()
      .to_vec();
    let clockwise = speed > 0;
    hardware_cmds.push(
      HardwareWriteCmd::new(
        &vec![LOVENSE_CONNECT_UUID],
        Endpoint::Tx,
        lovense_cmd,
        false,
      )
      .into(),
    );
    let dir = self.rotation_direction.load(Ordering::Relaxed);
    // TODO Should we store speed and direction as an option for rotation caching? This is weird.
    if dir != clockwise {
      self.rotation_direction.store(clockwise, Ordering::Relaxed);
      hardware_cmds.push(
        HardwareWriteCmd::new(
          &vec![LOVENSE_CONNECT_UUID],
          Endpoint::Tx,
          b"RotateChange?".to_vec(),
          false,
        )
        .into(),
      );
    }
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
