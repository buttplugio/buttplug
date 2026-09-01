// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! [`ProtocolHandler`] implementation backed by a compiled rhai protocol
//! script.

use std::sync::{Arc, Mutex};

use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use rhai::{AST, CallFnOptions, Dynamic, Engine, Map, Scope};
use std::str::FromStr;
use uuid::Uuid;

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{
    GenericProtocolIdentifier,
    ProtocolHandler,
    ProtocolIdentifier,
    ProtocolIdentifierFactory,
  },
};

use super::engine::script_engine;

/// Stable prefix used for all script protocol error messages so failures can
/// be attributed to the script that caused them.
fn error_prefix(protocol_name: &str) -> String {
  format!("Rhai protocol {protocol_name}: ")
}

/// A [`ProtocolHandler`] whose behavior is defined by a rhai protocol script.
///
/// Each device connection gets its own handler instance with its own `this`
/// state (a deep clone of the script's load-time-validated `init_state()`
/// template, or an empty map when the script has none). State persists across
/// handler calls for the lifetime of the connection only.
pub struct ScriptedProtocolHandler {
  engine: &'static Engine,
  ast: Arc<AST>,
  protocol_name: String,
  state: Mutex<Dynamic>,
}

impl ScriptedProtocolHandler {
  /// Creates a new handler. `state_template` must be a map (validated at
  /// script load time); it is deep-cloned so instances never share state.
  pub fn new(protocol_name: &str, ast: Arc<AST>, state_template: Dynamic) -> Self {
    Self {
      engine: script_engine(),
      ast,
      protocol_name: protocol_name.to_owned(),
      state: Mutex::new(state_template.flatten_clone()),
    }
  }

  fn script_error(&self, message: impl std::fmt::Display) -> ButtplugDeviceError {
    ButtplugDeviceError::DeviceSpecificError(format!(
      "{}{}",
      error_prefix(&self.protocol_name),
      message
    ))
  }

  fn has_script_fn(&self, fn_name: &str) -> bool {
    self.ast.iter_functions().any(|f| f.name == fn_name)
  }

  /// Calls a script function with the handler's state bound to `this`.
  fn call_script_fn(
    &self,
    fn_name: &str,
    args: Vec<Dynamic>,
  ) -> Result<Dynamic, ButtplugDeviceError> {
    let mut state = self
      .state
      .lock()
      .map_err(|_| self.script_error("internal state mutex poisoned by a previous script panic"))?;
    let mut options = CallFnOptions::new().bind_this_ptr(&mut *state);
    // Never evaluate the AST body: only the named function may run.
    options.eval_ast = false;
    let mut scope = Scope::new();
    self
      .engine
      .call_fn_with_options(options, &mut scope, &self.ast, fn_name, args)
      .map_err(|e| self.script_error(e))
  }

  /// Converts a script handler's return value (an array of command maps) into
  /// hardware commands, validating every field.
  fn convert_commands(
    &self,
    result: Dynamic,
    feature_id: Uuid,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let array = result
      .into_array()
      .map_err(|_| self.script_error("handler must return an array of command maps"))?;

    let mut commands = Vec::with_capacity(array.len());
    for (command_index, entry) in array.into_iter().enumerate() {
      let map: Map = entry.flatten_clone().try_cast().ok_or_else(|| {
        self.script_error(format!("command {command_index} is not an object map"))
      })?;

      // endpoint (required): string matching an Endpoint name.
      let endpoint_value = map
        .get("endpoint")
        .ok_or_else(|| self.script_error(format!("command {command_index} is missing endpoint")))?;
      let endpoint_str = endpoint_value
        .as_immutable_string_ref()
        .map_err(|_| {
          self.script_error(format!(
            "command {command_index} field endpoint must be a string"
          ))
        })?
        .to_string();
      let endpoint = Endpoint::from_str(&endpoint_str)
        .map_err(|_| ButtplugDeviceError::InvalidEndpoint(endpoint_str.clone()))?;

      // data (required): a Blob or an array of ints in 0..=255.
      let data_value = map
        .get("data")
        .ok_or_else(|| self.script_error(format!("command {command_index} is missing data")))?;
      let data = if let Ok(blob) = data_value.as_blob_ref() {
        blob.to_vec()
      } else if let Ok(array) = data_value.as_array_ref() {
        let mut bytes = Vec::with_capacity(array.len());
        for (byte_index, byte) in array.iter().enumerate() {
          let int_value = byte.as_int().map_err(|_| {
            self.script_error(format!(
              "command {command_index} data[{byte_index}] must be an integer"
            ))
          })?;
          if !(0..=255).contains(&int_value) {
            return Err(self.script_error(format!(
              "command {command_index} data[{byte_index}] value {int_value} is out of range 0..=255"
            )));
          }
          bytes.push(int_value as u8);
        }
        bytes
      } else {
        return Err(self.script_error(format!(
          "command {command_index} field data must be a Blob or an array of integers"
        )));
      };

      // write_with_response (optional, default false): must be a bool.
      let write_with_response = match map.get("write_with_response") {
        None => false,
        Some(value) => value.as_bool().map_err(|_| {
          self.script_error(format!(
            "command {command_index} field write_with_response must be a bool"
          ))
        })?,
      };

      // command_ids (optional, defaults to the handled feature's id): array of
      // UUID strings.
      let command_ids = match map.get("command_ids") {
        None => vec![feature_id],
        Some(value) => {
          let id_array = value.as_array_ref().map_err(|_| {
            self.script_error(format!(
              "command {command_index} field command_ids must be an array of UUID strings"
            ))
          })?;
          let mut ids = Vec::with_capacity(id_array.len());
          for (id_index, id_value) in id_array.iter().enumerate() {
            let id_str = id_value
              .as_immutable_string_ref()
              .map_err(|_| {
                self.script_error(format!(
                  "command {command_index} command_ids[{id_index}] must be a string"
                ))
              })?
              .as_str()
              .to_owned();
            let id = Uuid::parse_str(&id_str).map_err(|_| {
              self.script_error(format!(
                "command {command_index} command_ids[{id_index}] is not a valid UUID: {id_str}"
              ))
            })?;
            ids.push(id);
          }
          ids
        }
      };

      commands
        .push(HardwareWriteCmd::new(&command_ids, endpoint, data, write_with_response).into());
    }
    Ok(commands)
  }

  /// Shared body for the two-int-arg handlers.
  fn handle_two_args(
    &self,
    script_fn: &str,
    unimplemented_msg: &str,
    feature_index: u32,
    feature_id: Uuid,
    value: i64,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if !self.has_script_fn(script_fn) {
      return Err(ButtplugDeviceError::UnhandledCommand(
        unimplemented_msg.to_owned(),
      ));
    }
    let result = self.call_script_fn(
      script_fn,
      vec![
        Dynamic::from_int(feature_index as i64),
        Dynamic::from_int(value),
      ],
    )?;
    self.convert_commands(result, feature_id)
  }

  fn handle_three_args(
    &self,
    script_fn: &str,
    unimplemented_msg: &str,
    feature_index: u32,
    feature_id: Uuid,
    value: i64,
    duration: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if !self.has_script_fn(script_fn) {
      return Err(ButtplugDeviceError::UnhandledCommand(
        unimplemented_msg.to_owned(),
      ));
    }
    let result = self.call_script_fn(
      script_fn,
      vec![
        Dynamic::from_int(feature_index as i64),
        Dynamic::from_int(value),
        Dynamic::from_int(duration as i64),
      ],
    )?;
    self.convert_commands(result, feature_id)
  }
}

impl ProtocolHandler for ScriptedProtocolHandler {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.handle_two_args(
      "handle_vibrate",
      "Command not implemented for this protocol: OutputCmd (Vibrate Actuator)",
      feature_index,
      feature_id,
      speed as i64,
    )
  }

  fn handle_output_rotate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: i32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.handle_two_args(
      "handle_rotate",
      "Command not implemented for this protocol: OutputCmd (Rotate Actuator)",
      feature_index,
      feature_id,
      speed as i64,
    )
  }

  fn handle_output_oscillate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.handle_two_args(
      "handle_oscillate",
      "Command not implemented for this protocol: OutputCmd (Oscillate Actuator)",
      feature_index,
      feature_id,
      speed as i64,
    )
  }

  fn handle_output_spray_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.handle_two_args(
      "handle_spray",
      "Command not implemented for this protocol: OutputCmd (Spray Actuator)",
      feature_index,
      feature_id,
      level as i64,
    )
  }

  fn handle_output_constrict_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.handle_two_args(
      "handle_constrict",
      "Command not implemented for this protocol: OutputCmd (Constrict Actuator)",
      feature_index,
      feature_id,
      level as i64,
    )
  }

  fn handle_output_temperature_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    level: i32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.handle_two_args(
      "handle_temperature",
      "Command not implemented for this protocol: OutputCmd (Temperature Actuator)",
      feature_index,
      feature_id,
      level as i64,
    )
  }

  fn handle_output_led_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.handle_two_args(
      "handle_led",
      "Command not implemented for this protocol: OutputCmd (Led Actuator)",
      feature_index,
      feature_id,
      level as i64,
    )
  }

  fn handle_output_position_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    position: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.handle_two_args(
      "handle_position",
      "Command not implemented for this protocol: OutputCmd (Position Actuator)",
      feature_index,
      feature_id,
      position as i64,
    )
  }

  fn handle_hw_position_with_duration_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    position: u32,
    duration: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.handle_three_args(
      "handle_position_duration",
      "Command not implemented for this protocol: OutputCmd (Position w/ Duration Actuator)",
      feature_index,
      feature_id,
      position as i64,
      duration,
    )
  }
}

/// Factory creating a fresh [`ScriptedProtocolHandler`] (with fresh state) per
/// device connection. One factory exists per loaded script.
pub struct ScriptedProtocolFactory {
  protocol_name: String,
  ast: Arc<AST>,
  state_template: Dynamic,
}

impl std::fmt::Debug for ScriptedProtocolFactory {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    // ASTs are large; report just the identity.
    f.debug_struct("ScriptedProtocolFactory")
      .field("protocol_name", &self.protocol_name)
      .finish()
  }
}

impl ScriptedProtocolFactory {
  /// `state_template` must be a map; validated at script load time.
  pub fn new(protocol_name: &str, ast: Arc<AST>, state_template: Dynamic) -> Self {
    Self {
      protocol_name: protocol_name.to_owned(),
      ast,
      state_template,
    }
  }

  pub(crate) fn handler(&self) -> Arc<ScriptedProtocolHandler> {
    Arc::new(ScriptedProtocolHandler::new(
      &self.protocol_name,
      self.ast.clone(),
      self.state_template.flatten_clone(),
    ))
  }
}

impl ProtocolIdentifierFactory for ScriptedProtocolFactory {
  fn identifier(&self) -> &str {
    &self.protocol_name
  }

  fn create(&self) -> Box<dyn ProtocolIdentifier> {
    Box::new(GenericProtocolIdentifier::new(
      self.handler(),
      &self.protocol_name,
    ))
  }
}
