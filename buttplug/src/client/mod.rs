mod v3;
//mod v4;
pub mod connector;
pub mod serializer;

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
