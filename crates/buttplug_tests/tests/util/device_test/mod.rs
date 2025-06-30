// Since all of the uses of this module are generated, dead code resolution doesn't work.
#![allow(dead_code)]
pub mod client;
pub mod connector;
use super::{TestDeviceIdentifier, TestHardwareEvent};
use buttplug_server::device::hardware::HardwareCommand;
use buttplug_server::message::{
    RotationSubcommandV1,
    ScalarSubcommandV3,
    VectorSubcommandV1,
    VibrateSubcommandV1,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct TestDevice {
  identifier: TestDeviceIdentifier,
  expected_name: Option<String>,
  expected_display_name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
enum TestCommand {
  Messages {
    device_index: u32,
    messages: Vec<TestClientCommand>,
  },
  Commands {
    device_index: u32,
    commands: Vec<HardwareCommand>,
  },
  Events {
    device_index: u32,
    events: Vec<TestHardwareEvent>,
  },
}

#[derive(Serialize, Deserialize, Debug)]
enum TestClientCommand {
  Scalar(Vec<ScalarSubcommandV3>),
  Vibrate(Vec<VibrateSubcommandV1>),
  Rotate(Vec<RotationSubcommandV1>),
  Linear(Vec<VectorSubcommandV1>),
  Battery {
    expected_power: f64,
    run_async: bool,
  },
  Stop,
  RSSI,
}

#[derive(Serialize, Deserialize)]
pub struct DeviceTestCase {
  devices: Vec<TestDevice>,
  device_config_file: Option<String>,
  user_device_config_file: Option<String>,
  device_init: Option<Vec<TestCommand>>,
  device_commands: Vec<TestCommand>,
}
