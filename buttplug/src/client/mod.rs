mod v3;
pub mod v4;

#[cfg(not(feature = "default_v4_spec"))]
pub use v3::{
  device::{
    ButtplugClientDevice,
    ButtplugClientDeviceEvent,
    LinearCommand,
    RotateCommand,
    ScalarCommand,
    ScalarValueCommand,
  },
  ButtplugClient,
  ButtplugClientError,
  ButtplugClientEvent,
  serializer,
  connector,
};

#[cfg(feature = "default_v4_spec")]
pub use v4::{
  device::{
    ButtplugClientDevice,
    ButtplugClientDeviceEvent,
    LinearCommand,
    RotateCommand,
    ScalarCommand,
    ScalarValueCommand,
  },
  ButtplugClient,
  ButtplugClientError,
  ButtplugClientEvent,
};
