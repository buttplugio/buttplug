// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod lovense_dual_actuator;
mod lovense_max;
mod lovense_multi_actuator;
mod lovense_rotate_vibrator;
mod lovense_single_actuator;
mod lovense_stroker;

use lovense_dual_actuator::LovenseDualActuator;
use lovense_max::LovenseMax;
use lovense_multi_actuator::LovenseMultiActuator;
use lovense_rotate_vibrator::LovenseRotateVibrator;
use lovense_single_actuator::LovenseSingleActuator;
use lovense_stroker::LovenseStroker;

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareEvent, HardwareSubscribeCmd, HardwareWriteCmd},
  protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer, ProtocolKeepaliveStrategy},
};
use async_trait::async_trait;
use buttplug_core::{
  errors::ButtplugDeviceError,
  message::{self, InputReadingV4, InputTypeReading, InputValue, OutputType},
  util::sleep,
};
use buttplug_server_device_config::{
  Endpoint,
  ProtocolCommunicationSpecifier,
  ServerDeviceDefinition,
  UserDeviceIdentifier,
};
use futures::{FutureExt, future::BoxFuture};
use regex::Regex;
use std::{sync::Arc, time::Duration};
use tokio::select;
use uuid::{Uuid, uuid};

// Constants for dealing with the Lovense subscript/write race condition. The
// timeout needs to be VERY long, otherwise this trips up old lovense serial
// adapters.
//
// Just buy new adapters, people.
const LOVENSE_COMMAND_TIMEOUT_MS: u64 = 500;
const LOVENSE_COMMAND_RETRY: u64 = 5;

const LOVENSE_PROTOCOL_UUID: Uuid = uuid!("cfa3fac5-48bb-4d87-817e-a439965956e1");

pub mod setup {
  use crate::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};
  #[derive(Default)]
  pub struct LovenseIdentifierFactory {}

  impl ProtocolIdentifierFactory for LovenseIdentifierFactory {
    fn identifier(&self) -> &str {
      "lovense"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::LovenseIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct LovenseIdentifier {}

fn lovense_model_resolver(type_response: String) -> String {
  let parts = type_response.split(':').collect::<Vec<&str>>();
  if parts.len() < 2 {
    warn!(
      "Lovense Device returned invalid DeviceType info: {}",
      type_response
    );
    return "lovense".to_string();
  }

  let identifier = parts[0].to_owned();
  let version = parts[1].to_owned().parse::<i32>().unwrap_or(0);

  info!("Identified device type {} version {}", identifier, version);

  // Flexer: version must be 3+ to control actuators separately
  if identifier == "EI" && version >= 3 {
    return "EI-FW3".to_string();
  }

  identifier
}

#[async_trait]
impl ProtocolIdentifier for LovenseIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
    _: ProtocolCommunicationSpecifier,
  ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    let mut event_receiver = hardware.event_stream();
    let mut count = 0;
    hardware
      .subscribe(&HardwareSubscribeCmd::new(
        LOVENSE_PROTOCOL_UUID,
        Endpoint::Rx,
      ))
      .await?;

    loop {
      let msg = HardwareWriteCmd::new(
        &[LOVENSE_PROTOCOL_UUID],
        Endpoint::Tx,
        b"DeviceType;".to_vec(),
        false,
      );
      hardware.write_value(&msg).await?;

      select! {
        event = event_receiver.recv().fuse() => {
          if let Ok(HardwareEvent::Notification(_, _, n)) = event {
            let type_response = std::str::from_utf8(&n).map_err(|_| ButtplugDeviceError::ProtocolSpecificError("lovense".to_owned(), "Lovense device init got back non-UTF8 string.".to_owned()))?.to_owned();
            debug!("Lovense Device Type Response: {}", type_response);
            let ident = lovense_model_resolver(type_response);
            return Ok((UserDeviceIdentifier::new(hardware.address(), "lovense", &Some(ident.clone())), Box::new(LovenseInitializer::new(ident))));
          } else {
            return Err(
              ButtplugDeviceError::ProtocolSpecificError(
                "Lovense".to_owned(),
                "Lovense Device disconnected while getting DeviceType info.".to_owned(),
              ),
            );
          }
        }
        _ = sleep(Duration::from_millis(LOVENSE_COMMAND_TIMEOUT_MS)).fuse() => {
          count += 1;
          if count > LOVENSE_COMMAND_RETRY {
            warn!("Lovense Device timed out while getting DeviceType info. ({} retries)", LOVENSE_COMMAND_RETRY);
            let re = Regex::new(r"LVS-([A-Z]+)\d+").expect("Static regex shouldn't fail");
            if let Some(caps) = re.captures(hardware.name()) {
              info!("Lovense Device identified by BLE name");
              return Ok((UserDeviceIdentifier::new(hardware.address(), "lovense", &Some(caps[1].to_string())), Box::new(LovenseInitializer::new(caps[1].to_string()))));
            };
            return Ok((UserDeviceIdentifier::new(hardware.address(), "lovense", &None), Box::new(LovenseInitializer::new("".to_string()))));
          }
        }
      }
    }
  }
}
pub struct LovenseInitializer {
  device_type: String,
}

impl LovenseInitializer {
  pub fn new(device_type: String) -> Self {
    Self { device_type }
  }
}

#[async_trait]
impl ProtocolInitializer for LovenseInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    device_definition: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let device_type = self.device_type.clone();

    let vibrator_count = device_definition
      .features()
      .values()
      .filter(|x| {
        x.output()
          .as_ref()
          .is_some_and(|x| x.contains(OutputType::Vibrate) || x.contains(OutputType::Oscillate))
      })
      .count();

    let output_count = device_definition
      .features()
      .values()
      .filter(|x| x.output().is_some())
      .count();

    let vibrator_rotator = output_count == 2
      && device_definition
        .features()
        .values()
        .filter(|x| {
          x.output()
            .as_ref()
            .is_some_and(|x| x.contains(OutputType::Vibrate))
        })
        .count()
        == 1
      && device_definition
        .features()
        .values()
        .filter(|x| {
          x.output()
            .as_ref()
            .is_some_and(|x| x.contains(OutputType::Rotate))
        })
        .count()
        == 1;

    let lovense_max = output_count == 2
      && device_definition
        .features()
        .values()
        .filter(|x| {
          x.output()
            .as_ref()
            .is_some_and(|x| x.contains(OutputType::Vibrate))
        })
        .count()
        == 1
      && device_definition
        .features()
        .values()
        .filter(|x| {
          x.output()
            .as_ref()
            .is_some_and(|x| x.contains(OutputType::Constrict))
        })
        .count()
        == 1;

    // This might need better tuning if other complex Lovenses are released
    // Currently this only applies to the Flexer/Lapis/Solace
    let use_mply = (vibrator_count == 2 && output_count > 2) || vibrator_count > 2;

    // New Lovense devices seem to be moving to the simplified LVS:<bytearray>; command format.
    // I'm not sure if there's a good way to detect this.
    //let use_lvs = device_type == "OC";

    debug!(
      "Device type {} initialized with {} outputs {} using Mply",
      device_type,
      output_count,
      if use_mply { "" } else { "not " }
    );

    if device_type == "BA" || device_type == "H" {
      Ok(Arc::new(LovenseStroker::new(hardware, device_type == "H")))
    } else if output_count == 1 {
      Ok(Arc::new(LovenseSingleActuator::default()))
    } else if lovense_max {
      Ok(Arc::new(LovenseMax::default()))
    } else if vibrator_rotator {
      Ok(Arc::new(LovenseRotateVibrator::default()))
    } else if use_mply {
      Ok(Arc::new(LovenseMultiActuator::new(output_count as u32)))
    } else {
      Ok(Arc::new(LovenseDualActuator::new(output_count as u32)))
    }
  }
}

fn handle_battery_level_cmd(
  device_index: u32,
  device: Arc<Hardware>,
  feature_index: u32,
  feature_id: Uuid,
) -> BoxFuture<'static, Result<InputReadingV4, ButtplugDeviceError>> {
  let mut device_notification_receiver = device.event_stream();
  async move {
    let write_fut = device.write_value(&HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      b"Battery;".to_vec(),
      false,
    ));
    write_fut.await?;
    while let Ok(event) = device_notification_receiver.recv().await {
      match event {
        HardwareEvent::Notification(_, _, data) => {
          if let Ok(data_str) = std::str::from_utf8(&data) {
            debug!("Lovense event received: {}", data_str);
            let len = data_str.len();
            // Depending on the state of the toy, we may get an initial
            // character of some kind, i.e. if the toy is currently vibrating
            // then battery level comes up as "s89;" versus just "89;". We'll
            // need to chop the semicolon and make sure we only read the
            // numbers in the string.
            //
            // Contains() is casting a wider net than we need here, but it'll
            // do for now.
            let start_pos = usize::from(data_str.contains('s'));
            if let Ok(level) = data_str[start_pos..(len - 1)].parse::<u8>() {
              return Ok(message::InputReadingV4::new(
                device_index,
                feature_index,
                InputTypeReading::Battery(InputValue::new(level)),
              ));
            }
          }
        }
        HardwareEvent::Disconnected(_) => {
          return Err(ButtplugDeviceError::ProtocolSpecificError(
            "Lovense".to_owned(),
            "Lovense Device disconnected while getting Battery info.".to_owned(),
          ));
        }
      }
    }
    Err(ButtplugDeviceError::ProtocolSpecificError(
      "Lovense".to_owned(),
      "Lovense Device disconnected while getting Battery info.".to_owned(),
    ))
  }
  .boxed()
}

pub(super) fn keepalive_strategy() -> ProtocolKeepaliveStrategy {
  // For Lovense, we'll just repeat the device type packet and drop the result.
  ProtocolKeepaliveStrategy::HardwareRequiredRepeatPacketStrategy(HardwareWriteCmd::new(
    &[LOVENSE_PROTOCOL_UUID],
    Endpoint::Tx,
    b"DeviceType;".to_vec(),
    false,
  ))
}

pub(super) fn form_lovense_command(
  feature_id: Uuid,
  command: &str,
) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
  Ok(vec![
    HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      command.as_bytes().to_vec(),
      false,
    )
    .into(),
  ])
}

pub(super) fn form_vibrate_command(
  feature_id: Uuid,
  speed: u32,
) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
  form_lovense_command(feature_id, &format!("Vibrate:{speed};"))
}

// Due to swapping direction with lovense requiring a seperate command, we have to treat these like
// two seperate outputs, otherwise we'll stomp on ourselves. Luckily Lovense devices currently only
// have one rotation mechanism.
const LOVENSE_ROTATE_UUID: Uuid = uuid!("4a741489-922f-4f0b-a594-175b75482849");
const LOVENSE_ROTATE_DIRECTION_UUID: Uuid = uuid!("4ad23456-2ba8-4916-bd91-9b603811f253");

pub(super) fn form_rotate_with_direction_command(
  speed: u32,
  change_direction: bool,
) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
  let mut hardware_cmds = vec![];
  if change_direction {
    hardware_cmds.push(
      HardwareWriteCmd::new(
        &[LOVENSE_ROTATE_DIRECTION_UUID],
        Endpoint::Tx,
        b"RotateChange;".to_vec(),
        false,
      )
      .into(),
    );
  }
  let lovense_cmd = format!("Rotate:{speed};").as_bytes().to_vec();
  hardware_cmds
    .push(HardwareWriteCmd::new(&[LOVENSE_ROTATE_UUID], Endpoint::Tx, lovense_cmd, false).into());
  trace!("{:?}", hardware_cmds);
  Ok(hardware_cmds)
}
