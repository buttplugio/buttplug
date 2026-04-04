// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_server_device_config::{
  BaseDeviceIdentifier, Endpoint, ProtocolCommunicationSpecifier, ServerDeviceDefinition,
  ServerDeviceDefinitionBuilder, ServerDeviceFeature, ServerDeviceFeatureInput,
  ServerDeviceFeatureOutput, ServerDeviceFeatureOutputValueProperties, RangeWithLimit,
  BluetoothLESpecifier,
};
use std::collections::HashMap;
use uuid::Uuid;

/// A conformance test device definition with all required metadata
pub struct ConformanceDeviceDef {
  pub name: &'static str,
  pub address: String,
  pub endpoints: Vec<Endpoint>,
  pub base_identifier: BaseDeviceIdentifier,
  pub specifier: ProtocolCommunicationSpecifier,
  pub definition: ServerDeviceDefinition,
}

/// Returns the three canonical test devices for conformance testing
pub fn conformance_device_definitions() -> Vec<ConformanceDeviceDef> {
  vec![
    create_test_vibrator(),
    create_test_positioner(),
    create_test_multi(),
  ]
}

/// Device 0: "Conformance Test Vibrator"
/// Features:
/// - 0: Vibrator 1 (Output: Vibrate, 0-100)
/// - 1: Vibrator 2 (Output: Vibrate, 0-100)
/// - 2: Rotator (Output: Rotate, -100-100)
/// - 3: Battery (Input: Battery, Read, 0-100)
fn create_test_vibrator() -> ConformanceDeviceDef {
  let name = "Conformance Test Vibrator";
  let address = "test-vibrator-0".to_string();
  let endpoints = vec![Endpoint::Tx, Endpoint::RxBLEBattery];
  let base_id = BaseDeviceIdentifier::new_default("conformance");
  let device_id = Uuid::new_v4();

  let mut features = Vec::new();

  // Feature 0: Vibrator 1
  let mut output = ServerDeviceFeatureOutput::default();
  output.set_vibrate(Some(ServerDeviceFeatureOutputValueProperties::new(
    &RangeWithLimit::new(&(0..=100)),
    false,
  )));
  features.push(ServerDeviceFeature::new(
    0,
    "Vibrator 1",
    Uuid::new_v4(),
    None,
    None,
    &Some(output),
    &None,
  ));

  // Feature 1: Vibrator 2
  let mut output = ServerDeviceFeatureOutput::default();
  output.set_vibrate(Some(ServerDeviceFeatureOutputValueProperties::new(
    &RangeWithLimit::new(&(0..=100)),
    false,
  )));
  features.push(ServerDeviceFeature::new(
    1,
    "Vibrator 2",
    Uuid::new_v4(),
    None,
    None,
    &Some(output),
    &None,
  ));

  // Feature 2: Rotator
  let mut output = ServerDeviceFeatureOutput::default();
  output.set_rotate(Some(ServerDeviceFeatureOutputValueProperties::new(
    &RangeWithLimit::new(&(-100..=100)),
    false,
  )));
  features.push(ServerDeviceFeature::new(
    2,
    "Rotator",
    Uuid::new_v4(),
    None,
    None,
    &Some(output),
    &None,
  ));

  // Feature 3: Battery (Input)
  let input: ServerDeviceFeatureInput = serde_json::from_value(serde_json::json!({
    "battery": {
      "value": [[0, 100]],
      "command": ["Read"]
    }
  }))
  .expect("Valid battery input feature JSON");
  features.push(ServerDeviceFeature::new(
    3,
    "Battery",
    Uuid::new_v4(),
    None,
    None,
    &None,
    &Some(input),
  ));

  let mut builder = ServerDeviceDefinitionBuilder::new(name, &device_id);
  for feature in features {
    builder.add_feature(&feature);
  }
  let definition = builder.finish();

  let specifier = ProtocolCommunicationSpecifier::BluetoothLE(
    BluetoothLESpecifier::new_from_device(name, &HashMap::new(), &[]),
  );

  ConformanceDeviceDef {
    name,
    address,
    endpoints,
    base_identifier: base_id,
    specifier,
    definition,
  }
}

/// Device 1: "Conformance Test Positioner"
/// Features:
/// - 0: Position (Output: Position, 0-100)
/// - 1: Position w/ Duration (Output: HwPositionWithDuration, 0-100, duration 0-10000ms)
/// - 2: Oscillator (Output: Oscillate, 0-100)
/// - 3: Button (Input: Button, Subscribe, 0-1)
fn create_test_positioner() -> ConformanceDeviceDef {
  let name = "Conformance Test Positioner";
  let address = "test-positioner-1".to_string();
  let endpoints = vec![Endpoint::Tx, Endpoint::Generic1]; // Generic1 = Button
  let base_id = BaseDeviceIdentifier::new_default("conformance");
  let device_id = Uuid::new_v4();

  let mut features = Vec::new();

  // Feature 0: Position
  let mut output = ServerDeviceFeatureOutput::default();
  output.set_position(Some(
    buttplug_server_device_config::ServerDeviceFeatureOutputPositionProperties::new(
      &RangeWithLimit::new(&(0..=100)),
      false,
      false,
    ),
  ));
  features.push(ServerDeviceFeature::new(
    0,
    "Position",
    Uuid::new_v4(),
    None,
    None,
    &Some(output),
    &None,
  ));

  // Feature 1: Position w/ Duration
  let mut output = ServerDeviceFeatureOutput::default();
  output.set_hw_position_with_duration(Some(
    buttplug_server_device_config::ServerDeviceFeatureOutputHwPositionWithDurationProperties::new(
      &RangeWithLimit::new(&(0..=100)),
      &RangeWithLimit::new(&(0..=10000)),
      false,
      false,
    ),
  ));
  features.push(ServerDeviceFeature::new(
    1,
    "Position w/ Duration",
    Uuid::new_v4(),
    None,
    None,
    &Some(output),
    &None,
  ));

  // Feature 2: Oscillator
  let mut output = ServerDeviceFeatureOutput::default();
  output.set_oscillate(Some(ServerDeviceFeatureOutputValueProperties::new(
    &RangeWithLimit::new(&(0..=100)),
    false,
  )));
  features.push(ServerDeviceFeature::new(
    2,
    "Oscillator",
    Uuid::new_v4(),
    None,
    None,
    &Some(output),
    &None,
  ));

  // Feature 3: Button (Input)
  let input: ServerDeviceFeatureInput = serde_json::from_value(serde_json::json!({
    "button": {
      "value": [[0, 1]],
      "command": ["Subscribe"]
    }
  }))
  .expect("Valid button input feature JSON");
  features.push(ServerDeviceFeature::new(
    3,
    "Button",
    Uuid::new_v4(),
    None,
    None,
    &None,
    &Some(input),
  ));

  let mut builder = ServerDeviceDefinitionBuilder::new(name, &device_id);
  for feature in features {
    builder.add_feature(&feature);
  }
  let definition = builder.finish();

  let specifier = ProtocolCommunicationSpecifier::BluetoothLE(
    BluetoothLESpecifier::new_from_device(name, &HashMap::new(), &[]),
  );

  ConformanceDeviceDef {
    name,
    address,
    endpoints,
    base_identifier: base_id,
    specifier,
    definition,
  }
}

/// Device 2: "Conformance Test Multi"
/// Features:
/// - 0: Constrictor (Output: Constrict, 0-100)
/// - 1: Sprayer (Output: Spray, 0-100)
/// - 2: Heater (Output: Temperature, -100-100)
/// - 3: LED (Output: Led, 0-100)
/// - 4: RSSI (Input: Rssi, Read, -128-0)
/// - 5: Pressure (Input: Pressure, Subscribe, 0-65535)
fn create_test_multi() -> ConformanceDeviceDef {
  let name = "Conformance Test Multi";
  let address = "test-multi-2".to_string();
  let endpoints = vec![Endpoint::Tx, Endpoint::Generic0, Endpoint::Generic2]; // Generic0 = Rssi, Generic2 = Pressure
  let base_id = BaseDeviceIdentifier::new_default("conformance");
  let device_id = Uuid::new_v4();

  let mut features = Vec::new();

  // Feature 0: Constrictor
  let mut output = ServerDeviceFeatureOutput::default();
  output.set_constrict(Some(ServerDeviceFeatureOutputValueProperties::new(
    &RangeWithLimit::new(&(0..=100)),
    false,
  )));
  features.push(ServerDeviceFeature::new(
    0,
    "Constrictor",
    Uuid::new_v4(),
    None,
    None,
    &Some(output),
    &None,
  ));

  // Feature 1: Sprayer
  let mut output = ServerDeviceFeatureOutput::default();
  output.set_spray(Some(ServerDeviceFeatureOutputValueProperties::new(
    &RangeWithLimit::new(&(0..=100)),
    false,
  )));
  features.push(ServerDeviceFeature::new(
    1,
    "Sprayer",
    Uuid::new_v4(),
    None,
    None,
    &Some(output),
    &None,
  ));

  // Feature 2: Heater
  let mut output = ServerDeviceFeatureOutput::default();
  output.set_temperature(Some(ServerDeviceFeatureOutputValueProperties::new(
    &RangeWithLimit::new(&(-100..=100)),
    false,
  )));
  features.push(ServerDeviceFeature::new(
    2,
    "Heater",
    Uuid::new_v4(),
    None,
    None,
    &Some(output),
    &None,
  ));

  // Feature 3: LED
  let mut output = ServerDeviceFeatureOutput::default();
  output.set_led(Some(ServerDeviceFeatureOutputValueProperties::new(
    &RangeWithLimit::new(&(0..=100)),
    false,
  )));
  features.push(ServerDeviceFeature::new(
    3,
    "LED",
    Uuid::new_v4(),
    None,
    None,
    &Some(output),
    &None,
  ));

  // Feature 4: RSSI (Input)
  let input: ServerDeviceFeatureInput = serde_json::from_value(serde_json::json!({
    "rssi": {
      "value": [[-128, 0]],
      "command": ["Read"]
    }
  }))
  .expect("Valid rssi input feature JSON");
  features.push(ServerDeviceFeature::new(
    4,
    "RSSI",
    Uuid::new_v4(),
    None,
    None,
    &None,
    &Some(input),
  ));

  // Feature 5: Pressure (Input)
  let input: ServerDeviceFeatureInput = serde_json::from_value(serde_json::json!({
    "pressure": {
      "value": [[0, 65535]],
      "command": ["Subscribe"]
    }
  }))
  .expect("Valid pressure input feature JSON");
  features.push(ServerDeviceFeature::new(
    5,
    "Pressure",
    Uuid::new_v4(),
    None,
    None,
    &None,
    &Some(input),
  ));

  let mut builder = ServerDeviceDefinitionBuilder::new(name, &device_id);
  for feature in features {
    builder.add_feature(&feature);
  }
  let definition = builder.finish();

  let specifier = ProtocolCommunicationSpecifier::BluetoothLE(
    BluetoothLESpecifier::new_from_device(name, &HashMap::new(), &[]),
  );

  ConformanceDeviceDef {
    name,
    address,
    endpoints,
    base_identifier: base_id,
    specifier,
    definition,
  }
}
