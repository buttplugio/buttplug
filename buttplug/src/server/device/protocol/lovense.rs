// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{self, Endpoint, FeatureType, SensorReadingV4},
  },
  server::device::{
    configuration::{ProtocolCommunicationSpecifier, UserDeviceDefinition, UserDeviceIdentifier},
    hardware::{Hardware, HardwareCommand, HardwareEvent, HardwareSubscribeCmd, HardwareWriteCmd},
    protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer},
  },
  util::{async_manager, sleep},
};
use async_trait::async_trait;
use dashmap::DashMap;
use futures::{future::BoxFuture, FutureExt};
use regex::Regex;
use std::{
  sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc,
  },
  time::Duration,
};
use uuid::{uuid, Uuid};

use super::ProtocolCommandOutputStrategy;

// Constants for dealing with the Lovense subscript/write race condition. The
// timeout needs to be VERY long, otherwise this trips up old lovense serial
// adapters.
//
// Just buy new adapters, people.
const LOVENSE_COMMAND_TIMEOUT_MS: u64 = 500;
const LOVENSE_COMMAND_RETRY: u64 = 5;

const LOVENSE_PROTOCOL_UUID: Uuid = uuid!("cfa3fac5-48bb-4d87-817e-a439965956e1");

pub mod setup {
  use crate::server::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};
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
        LOVENSE_PROTOCOL_UUID,
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
    device_definition: &UserDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let device_type = self.device_type.clone();

    let vibrator_count = device_definition
      .features()
      .iter()
      .filter(|x| [FeatureType::Vibrate, FeatureType::Oscillate].contains(&x.feature_type()))
      .count();

    let actuator_count = device_definition
      .features()
      .iter()
      .filter(|x| x.actuator().is_some())
      .count();

    // This might need better tuning if other complex Lovenses are released
    // Currently this only applies to the Flexer/Lapis/Solace
    let use_mply =
      (vibrator_count == 2 && actuator_count > 2) || vibrator_count > 2 || device_type == "H";

    // New Lovense devices seem to be moving to the simplified LVS:<bytearray>; command format.
    // I'm not sure if there's a good way to detect this.
    let use_lvs = device_type == "OC";

    debug!(
      "Device type {} initialized with {} vibrators {} using Mply",
      device_type,
      vibrator_count,
      if use_mply { "" } else { "not " }
    );

    Ok(Arc::new(Lovense::new(
      hardware,
      &device_type,
      vibrator_count,
      use_mply,
      use_lvs,
    )))
  }
}

pub struct Lovense {
  rotation_direction: AtomicBool,
  vibrator_values: Vec<AtomicU32>,
  use_mply: bool,
  use_lvs: bool,
  device_type: String,
  value_cache: DashMap<Uuid, u32>,
  linear_info: Arc<(AtomicU32, AtomicU32)>,
}

impl Lovense {
  pub fn new(
    hardware: Arc<Hardware>,
    device_type: &str,
    vibrator_count: usize,
    use_mply: bool,
    use_lvs: bool,
  ) -> Self {
    let linear_info = Arc::new((AtomicU32::new(0), AtomicU32::new(0)));
    if device_type == "BA" {
      async_manager::spawn(update_linear_movement(
        hardware.clone(),
        linear_info.clone(),
      ));
    }

    let mut vibrator_values = vec![];
    for _ in 0..vibrator_count {
      vibrator_values.push(AtomicU32::new(0));
    }

    Self {
      rotation_direction: AtomicBool::new(false),
      vibrator_values,
      use_mply,
      use_lvs,
      device_type: device_type.to_owned(),
      value_cache: DashMap::new(),
      linear_info,
    }
  }

  fn handle_lvs_cmd(&self) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut speeds = "LVS:{}".as_bytes().to_vec();
    for i in self.vibrator_values.iter() {
      speeds.push(i.load(Ordering::Relaxed) as u8);
    }
    speeds.push(0x3b);

    Ok(vec![HardwareWriteCmd::new(
      LOVENSE_PROTOCOL_UUID,
      Endpoint::Tx,
      speeds,
      false,
    )
    .into()])
  }

  fn handle_mply_cmd(&self) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    /*
    let mut speeds = cmds
      .iter()
      .map(|x| {
        if let Some(val) = x {
          val.1.to_string()
        } else {
          "-1".to_string()
        }
      })
      .collect::<Vec<_>>();

    if speeds.len() == 1 && self.device_type == "H" {
      // Max range unless stopped
      speeds.push(if speeds[0] == "0" {
        "0".to_string()
      } else {
        "20".to_string()
      });
    }

    let lovense_cmd = format!("Mply:{};", speeds.join(":")).as_bytes().to_vec();

    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      lovense_cmd,
      false,
    )
    .into()])
    */
    Ok(vec![])
  }
}

impl ProtocolHandler for Lovense {
  fn cache_strategy(&self) -> ProtocolCommandOutputStrategy {
    ProtocolCommandOutputStrategy::FullCommand
  }

  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    // For Lovense, we'll just repeat the device type packet and drop the result.
    super::ProtocolKeepaliveStrategy::RepeatPacketStrategy(HardwareWriteCmd::new(
      LOVENSE_PROTOCOL_UUID,
      Endpoint::Tx,
      b"DeviceType;".to_vec(),
      false,
    ))
  }

  fn handle_actuator_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let current_vibrator_value =
      self.vibrator_values[feature_index as usize].load(Ordering::Relaxed);
    if current_vibrator_value == speed {
      Ok(vec![])
    } else {
      self.vibrator_values[feature_index as usize].store(speed, Ordering::Relaxed);
      let speeds: Vec<u32> = self
        .vibrator_values
        .iter()
        .map(|v| v.load(Ordering::Relaxed))
        .collect();
      if self.use_lvs {
        self.handle_lvs_cmd()
      } else if self.use_mply {
        self.handle_mply_cmd()
      } else {
        let lovense_cmd = if self.vibrator_values.len() == 1 {
          format!("Vibrate:{speed};").as_bytes().to_vec()
        } else {
          format!("Vibrate{}:{};", feature_index + 1, speed)
            .as_bytes()
            .to_vec()
        };
        Ok(vec![HardwareWriteCmd::new(
          feature_id,
          Endpoint::Tx,
          lovense_cmd,
          false,
        )
        .into()])
      }
    }
    /*
    if self.use_lvs {
      self.handle_lvs_cmd(cmd)
    } else if self.use_mply {
      self.handle_mply_cmd(cmd)
    } else {
      // Handle vibration commands, these will be by far the most common. Fucking machine oscillation
      // uses lovense vibrate commands internally too, so we can include them here.
      let vibrate_cmds: Vec<> = cmds
        .iter()
        .filter(|x| {
          if let Some(val) = x {
            [ActuatorType::Vibrate, ActuatorType::Oscillate].contains(&val.0)
          } else {
            false
          }
        })
        .map(|x| x.as_ref().expect("Already verified is some"))
        .collect();
            if !vibrate_cmds.is_empty() {
      // Lovense is the same situation as the Lovehoney Desire, where commands
      // are different if we're addressing all motors or seperate motors.
      // Difference here being that there's Lovense variants with different
      // numbers of motors.
      //
      // Neat way of checking if everything is the same via
      // https://sts10.github.io/2019/06/06/is-all-equal-function.html.
      //
      // Just make sure we're not matching on None, 'cause if that's the case
      // we ain't got shit to do.
      //
      // Note that the windowed comparison causes mixed types as well as mixed
      // speeds to fall back to separate commands. This is because the Gravity's
      // thruster on Vibrate2 is independent of Vibrate
      if self.vibrator_count == vibrate_cmds.len()
        && (self.vibrator_count == 1
          || vibrate_cmds
            .windows(2)
            .all(|w| w[0].0 == w[1].0 && w[0].1 == w[1].1))
      {
        let lovense_cmd = format!("Vibrate:{};", vibrate_cmds[0].1)
          .as_bytes()
          .to_vec();
        hardware_cmds.push(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false).into());
      } else {
        for (i, cmd) in cmds.iter().enumerate() {
          if let Some((actuator, speed)) = cmd {
            if ![ActuatorType::Vibrate, ActuatorType::Oscillate].contains(actuator) {
              continue;
            }
            let lovense_cmd = format!("Vibrate{}:{};", i + 1, speed).as_bytes().to_vec();
            hardware_cmds.push(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false).into());
          }
        }
      }
    }
      */
  }

  fn handle_actuator_constrict_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let lovense_cmd = format!("Air:Level:{level};").as_bytes().to_vec();

    Ok(vec![HardwareWriteCmd::new(
      feature_id,
      Endpoint::Tx,
      lovense_cmd,
      false,
    )
    .into()])
  }

  fn handle_actuator_rotate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.handle_rotation_with_direction_cmd(feature_index, feature_id, speed, false)
  }

  fn handle_rotation_with_direction_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
    clockwise: bool,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut hardware_cmds = vec![];
    let lovense_cmd = format!("Rotate:{speed};").as_bytes().to_vec();
    hardware_cmds.push(HardwareWriteCmd::new(feature_id, Endpoint::Tx, lovense_cmd, false).into());
    let current_dir = self.rotation_direction.load(Ordering::Relaxed);
    if current_dir != clockwise {
      self.rotation_direction.store(clockwise, Ordering::Relaxed);
      hardware_cmds.push(
        HardwareWriteCmd::new(feature_id, Endpoint::Tx, b"RotateChange;".to_vec(), false).into(),
      );
    }
    trace!("{:?}", hardware_cmds);
    Ok(hardware_cmds)
  }

  fn handle_battery_level_cmd(
    &self,
    device: Arc<Hardware>,
    feature_index: u32,
    feature_id: Uuid,
  ) -> BoxFuture<Result<SensorReadingV4, ButtplugDeviceError>> {
    let mut device_notification_receiver = device.event_stream();
    async move {
      let write_fut = device.write_value(&HardwareWriteCmd::new(
        feature_id,
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
                return Ok(message::SensorReadingV4::new(
                  0,
                  feature_index,
                  message::SensorType::Battery,
                  vec![level as i32],
                ));
              }
            }
          }
          HardwareEvent::Disconnected(_) => {
            return Err(ButtplugDeviceError::ProtocolSpecificError(
              "Lovense".to_owned(),
              "Lovense Device disconnected while getting Battery info.".to_owned(),
            ))
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

  fn handle_position_with_duration_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    position: u32,
    duration: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.linear_info.0.store(position, Ordering::Relaxed);
    self.linear_info.1.store(duration, Ordering::Relaxed);
    Ok(vec![])
  }
}

async fn update_linear_movement(device: Arc<Hardware>, linear_info: Arc<(AtomicU32, AtomicU32)>) {
  let mut last_goal_position = 0i32;
  let mut current_move_amount = 0i32;
  let mut current_position = 0i32;
  loop {
    // See if we've updated our goal position
    let goal_position = linear_info.0.load(Ordering::Relaxed) as i32;
    // If we have and it's not the same, recalculate based on current status.
    if last_goal_position != goal_position {
      last_goal_position = goal_position;
      // We move every 100ms, so divide the movement into that many chunks.
      // If we're moving so fast it'd be under our 100ms boundary, just move in 1 step.
      let move_steps = (linear_info.1.load(Ordering::Relaxed) / 100).max(1);
      current_move_amount = (goal_position - current_position) / move_steps as i32;
    }

    // If we aren't going anywhere, just pause then restart
    if current_position == last_goal_position {
      sleep(Duration::from_millis(100)).await;
      continue;
    }

    // Update our position, make sure we don't overshoot
    current_position += current_move_amount;
    if current_move_amount < 0 {
      if current_position < last_goal_position {
        current_position = last_goal_position;
      }
    } else if current_position > last_goal_position {
      current_position = last_goal_position;
    }

    let lovense_cmd = format!("FSetSite:{current_position};");

    let hardware_cmd = HardwareWriteCmd::new(
      LOVENSE_PROTOCOL_UUID,
      Endpoint::Tx,
      lovense_cmd.into_bytes(),
      false,
    );
    if device.write_value(&hardware_cmd).await.is_err() {
      return;
    }
    sleep(Duration::from_millis(100)).await;
  }
}
