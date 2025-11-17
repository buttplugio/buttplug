// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Handles client sessions, as well as discovery and communication with hardware.
//!
//! The Buttplug Server is just a thin frontend for device connection and communication. The server
//! itself doesn't do much other than configuring the device system and handling a few non-device
//! related tasks like [initial connection
//! handshake](https://buttplug-spec.docs.buttplug.io/architecture.html#stages) and system timeouts.
//! Once a connection is made from a [ButtplugClient](crate::client::ButtplugClient) to a
//! [ButtplugServer], the server mostly acts as a pass-thru frontend to the [DeviceManager].
//!
//! ## Server Lifetime
//!
//! The server has following lifetime stages:
//!
//! - Configuration
//!   - This happens across the [ButtplugServerBuilder], as well as the [ButtplugServer] instance it
//!     returns. During this time, we can specify attributes of the server like its name and if it
//!     will have a ping timer. It also allows for addition of protocols and device configurations
//!     to the system, either via configuration files or through manual API calls.
//! - Connection
//!   - After configuration is done, the server can be put into a listening mode (assuming
//!     [RemoteServer](ButtplugRemoteServer) is being used. for [in-process
//!     servers](crate::connector::ButtplugInProcessClientConnector), the client own the server and just
//!     connects to it directly). At this point, a [ButtplugClient](crate::client::ButtplugClient)
//!     can connect and start the
//!     [handshake](https://buttplug-spec.docs.buttplug.io/architecture.html#stages) process.
//! - Pass-thru
//!   - Once the handshake has succeeded, the server basically becomes a pass-thru to the
//!     [DeviceManager], which manages discovery of and communication with devices. The only thing
//!     the server instance manages at this point is ownership of the [DeviceManager] and
//!     ping timer, but doesn't really do much itself. The server remains in this state until the
//!     connection to the client is severed, at which point all devices connected to the device
//!     manager will be stopped.
//! - Disconnection
//!   - The server can be put back in Connection mode without being recreated after disconnection,
//!     to listen for another client connection while still maintaining connection to whatever
//!     devices the [DeviceManager] has.
//! - Destruction
//!   - If the server object is dropped, all devices are stopped and disconnected as part
//!     of the [DeviceManager] teardown.

#[macro_use]
extern crate log;

#[macro_use]
extern crate buttplug_derive;

#[macro_use]
extern crate strum_macros;

pub mod connector;
pub mod device;
pub mod message;
mod ping_timer;
mod server;
mod server_builder;
mod server_message_conversion;

pub use server::ButtplugServer;
pub use server_builder::ButtplugServerBuilder;

use futures::future::BoxFuture;
use thiserror::Error;

use buttplug_core::{
  errors::{ButtplugDeviceError, ButtplugError},
  message::ButtplugServerMessageV4,
};

/// Result type for Buttplug Server methods, as the server will always communicate in
/// [ButtplugServerMessage] instances in order to follow the [Buttplug
/// Spec](http://buttplug-spec.docs.buttplug.io).
pub type ButtplugServerResult = Result<ButtplugServerMessageV4, ButtplugError>;
/// Future type for Buttplug Server futures, as the server will always communicate in
/// [ButtplugServerMessage] instances in order to follow the [Buttplug
/// Spec](http://buttplug-spec.docs.buttplug.io).
pub type ButtplugServerResultFuture = BoxFuture<'static, ButtplugServerResult>;

/// Error enum for Buttplug Server configuration errors.
#[derive(Error, Debug)]
pub enum ButtplugServerError {
  /// DeviceConfigurationManager could not be built.
  #[error("The DeviceConfigurationManager could not be built: {0}")]
  DeviceConfigurationManagerError(ButtplugDeviceError),
  /// DeviceCommunicationManager type has already been added to the system.
  #[error("DeviceCommunicationManager of type {0} has already been added.")]
  DeviceCommunicationManagerTypeAlreadyAdded(String),
  /// Protocol has already been added to the system.
  #[error("Buttplug Protocol of type {0} has already been added to the system.")]
  ProtocolAlreadyAdded(String),
  /// Requested protocol has not been registered with the system.
  #[error("Buttplug Protocol of type {0} does not exist in the system and cannot be removed.")]
  ProtocolDoesNotExist(String),
}
