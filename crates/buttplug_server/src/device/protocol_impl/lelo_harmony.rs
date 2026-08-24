// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::{
    Hardware, HardwareCommand, HardwareEvent, HardwareSubscribeCmd, HardwareUnsubscribeCmd,
    HardwareWriteCmd,
  },
  protocol::{
    ProtocolHandler, ProtocolIdentifier, ProtocolInitializer, generic_protocol_initializer_setup,
  },
};
use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_core::util::async_manager;
use buttplug_server_device_config::Endpoint;
use buttplug_server_device_config::{
  ProtocolCommunicationSpecifier, ServerDeviceDefinition, UserDeviceIdentifier,
};
use std::{
  collections::HashMap,
  sync::{Arc, Mutex},
  time::Duration,
};
use uuid::{Uuid, uuid};

const LELO_HARMONY_PROTOCOL_UUID: Uuid = uuid!("220e180a-e6d5-4fd1-963e-43a6f990b717");
const LELO_HARMONY_F1SV3_VARIANT: &str = "f1sv3";
const LELO_F1SV3_DEFAULT_IDLE_STOP_TIMEOUT_MS: u32 = 800;
generic_protocol_initializer_setup!(LeloHarmony, "lelo-harmony");

#[derive(Default)]
pub struct LeloHarmonyInitializer {}

#[async_trait]
impl ProtocolInitializer for LeloHarmonyInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    def: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    // The Lelo Harmony has a very specific pairing flow:
    // * First the device is turned on in BLE mode (long press)
    // * Then the security endpoint (Whitelist) needs to be read (which we can do via subscribe)
    // * If it returns 0x00,00,00,00,00,00,00,00 the connection isn't not authorised
    // * To authorize, the password must be writen to the characteristic.
    // * If the password is unknown (buttplug lacks a storage mechanism right now), the power button
    //   must be pressed to send the password
    // * The password must not be sent whilst subscribed to the endpoint
    // * Once the password has been sent, the endpoint can be read for status again
    // * If it returns 0x00,00,00,00,00,00,00,00 the connection is authorised
    let mut event_receiver = hardware.event_stream();
    hardware
      .subscribe(&HardwareSubscribeCmd::new(
        LELO_HARMONY_PROTOCOL_UUID,
        Endpoint::Whitelist,
      ))
      .await?;

    loop {
      let event = event_receiver.recv().await;
      if let Ok(HardwareEvent::Notification(_, _, n)) = event {
        if n.iter().all(|b| *b == 0u8) {
          info!(
            "Lelo Harmony isn't authorised: Tap the device's power button to complete connection."
          )
        } else if !n.is_empty() && n[0] == 1u8 && n[1..].iter().all(|b| *b == 0u8) {
          debug!("Lelo Harmony is authorised!");
          if def
            .protocol_variant()
            .as_deref()
            .is_some_and(|variant| variant == LELO_HARMONY_F1SV3_VARIANT)
          {
            return Ok(Arc::new(LeloHarmony::f1sv3_harmony(hardware.clone(), def)));
          }
          return Ok(Arc::new(LeloHarmony::default()));
        } else {
          debug!("Lelo Harmony gave us a password: {:?}", n);
          // Can't send whilst subscribed
          hardware
            .unsubscribe(&HardwareUnsubscribeCmd::new(
              LELO_HARMONY_PROTOCOL_UUID,
              Endpoint::Whitelist,
            ))
            .await?;
          // Send with response
          hardware
            .write_value(&HardwareWriteCmd::new(
              &[LELO_HARMONY_PROTOCOL_UUID],
              Endpoint::Whitelist,
              n,
              true,
            ))
            .await?;
          // Get back to the loop
          hardware
            .subscribe(&HardwareSubscribeCmd::new(
              LELO_HARMONY_PROTOCOL_UUID,
              Endpoint::Whitelist,
            ))
            .await?;
        }
      } else {
        return Err(ButtplugDeviceError::ProtocolSpecificError(
          "LeloHarmony".to_owned(),
          "Lelo Harmony didn't provided valid security handshake".to_owned(),
        ));
      }
    }
  }
}

pub struct LeloHarmony {
  output_endpoint: Endpoint,
  use_zero_pattern_for_stop: bool,
  write_with_response: bool,
  idle_stop_timeout: Option<Duration>,
  hardware: Option<Arc<Hardware>>,
  state: Arc<Mutex<HashMap<u32, MotorState>>>,
}

#[derive(Default)]
struct MotorState {
  has_seen_nonzero: bool,
  generation: u64,
}

impl Default for LeloHarmony {
  fn default() -> Self {
    Self::new(Endpoint::Tx, false, false, None, None)
  }
}

impl LeloHarmony {
  pub(super) fn f1sv3(hardware: Arc<Hardware>, def: &ServerDeviceDefinition) -> Self {
    Self::new(
      Endpoint::TxVibrate,
      true,
      true,
      Self::idle_stop_timeout(def),
      Some(hardware),
    )
  }

  fn f1sv3_harmony(hardware: Arc<Hardware>, def: &ServerDeviceDefinition) -> Self {
    Self::new(
      Endpoint::Tx,
      true,
      true,
      Self::idle_stop_timeout(def),
      Some(hardware),
    )
  }

  fn idle_stop_timeout(def: &ServerDeviceDefinition) -> Option<Duration> {
    def.vibrate_smoothing_enabled().then(|| {
      Duration::from_millis(
        def
          .vibrate_smoothing_idle_stop_ms()
          .unwrap_or(LELO_F1SV3_DEFAULT_IDLE_STOP_TIMEOUT_MS) as u64,
      )
    })
  }

  fn new(
    output_endpoint: Endpoint,
    use_zero_pattern_for_stop: bool,
    write_with_response: bool,
    idle_stop_timeout: Option<Duration>,
    hardware: Option<Arc<Hardware>>,
  ) -> Self {
    Self {
      output_endpoint,
      use_zero_pattern_for_stop,
      write_with_response,
      idle_stop_timeout,
      hardware,
      state: Arc::new(Mutex::new(HashMap::new())),
    }
  }

  fn command_for_speed(
    &self,
    feature_id: Uuid,
    feature_index: u32,
    speed: u32,
  ) -> HardwareWriteCmd {
    let pattern = if self.use_zero_pattern_for_stop && speed == 0 {
      0x00
    } else {
      0x08
    };
    HardwareWriteCmd::new(
      &[feature_id],
      self.output_endpoint,
      vec![
        0x0a,
        0x12,
        feature_index as u8 + 1,
        pattern,
        0x00,
        0x00,
        0x00,
        0x00,
        speed as u8,
        0x00,
      ],
      self.write_with_response,
    )
  }

  fn maybe_defer_stop(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<bool, ButtplugDeviceError> {
    if speed != 0 {
      if self.idle_stop_timeout.is_some() {
        let mut state = self.state.lock().map_err(|_| {
          ButtplugDeviceError::ProtocolSpecificError(
            "LeloHarmony".to_owned(),
            "Lelo Harmony motor state lock failed".to_owned(),
          )
        })?;
        let motor_state = state.entry(feature_index).or_default();
        motor_state.has_seen_nonzero = true;
        motor_state.generation += 1;
      }
      return Ok(false);
    }

    let Some(idle_stop_timeout) = self.idle_stop_timeout else {
      return Ok(false);
    };
    let Some(hardware) = self.hardware.clone() else {
      return Ok(false);
    };

    let mut state = self.state.lock().map_err(|_| {
      ButtplugDeviceError::ProtocolSpecificError(
        "LeloHarmony".to_owned(),
        "Lelo Harmony motor state lock failed".to_owned(),
      )
    })?;
    let motor_state = state.entry(feature_index).or_default();
    if !motor_state.has_seen_nonzero {
      return Ok(true);
    }

    motor_state.generation += 1;
    let generation = motor_state.generation;
    let stop_cmd = self.command_for_speed(feature_id, feature_index, 0);

    let state = self.state.clone();
    buttplug_core::spawn!("LeloHarmonyF1sV3DelayedStop", async move {
      async_manager::sleep(idle_stop_timeout).await;
      let should_stop = state
        .lock()
        .ok()
        .and_then(|state| {
          state
            .get(&feature_index)
            .map(|state| state.generation == generation)
        })
        .unwrap_or(false);
      if should_stop {
        let _ = hardware.write_value(&stop_cmd).await;
      }
    });

    Ok(true)
  }

  fn handle_input_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if self.maybe_defer_stop(feature_index, feature_id, speed)? {
      return Ok(vec![]);
    }
    Ok(vec![
      self
        .command_for_speed(feature_id, feature_index, speed)
        .into(),
    ])
  }
}

impl ProtocolHandler for LeloHarmony {
  fn handle_output_rotate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: i32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.handle_input_cmd(feature_index, feature_id, speed as u32)
  }

  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.handle_input_cmd(feature_index, feature_id, speed)
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn uses_configured_output_endpoint_for_vibration() {
    let handler = LeloHarmony::new(Endpoint::TxVibrate, false, false, None, None);
    let commands = handler
      .handle_output_vibrate_cmd(1, uuid!("00000000-0000-0000-0000-000000000001"), 50)
      .expect("Command should build");

    assert_eq!(commands.len(), 1);
    match &commands[0] {
      HardwareCommand::Write(cmd) => {
        assert_eq!(cmd.endpoint(), Endpoint::TxVibrate);
        assert_eq!(
          cmd.data(),
          &[0x0a, 0x12, 0x02, 0x08, 0x00, 0x00, 0x00, 0x00, 0x32, 0x00]
        );
        assert!(!cmd.write_with_response());
      }
      _ => panic!("Expected write command"),
    }
  }

  #[test]
  fn can_use_zero_pattern_for_stop() {
    let handler = LeloHarmony::new(Endpoint::TxVibrate, true, true, None, None);
    let commands = handler
      .handle_output_vibrate_cmd(0, uuid!("00000000-0000-0000-0000-000000000001"), 0)
      .expect("Command should build");

    assert_eq!(commands.len(), 1);
    match &commands[0] {
      HardwareCommand::Write(cmd) => {
        assert_eq!(cmd.endpoint(), Endpoint::TxVibrate);
        assert_eq!(
          cmd.data(),
          &[0x0a, 0x12, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
        assert!(cmd.write_with_response());
      }
      _ => panic!("Expected write command"),
    }
  }
}
