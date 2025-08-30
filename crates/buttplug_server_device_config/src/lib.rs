// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Management of protocol and device hardware configurations
//!
//! Buttplug can handle device communication over several different mediums, including bluetooth,
//! usb, serial, various network protocols, and others. The library also provides multiple protocols
//! to communicate with this hardware. All of this information is stored in the
//! [DeviceConfigurationManager] (aka the DCM), a structure that is built whenever a [buttplug
//! server](crate::server::ButtplugServer) instance is created, and which is immutable for the life
//! of the server instance.
//!
//! The [DeviceConfigurationManager]'s main job is to take a newly discovered piece of hardware and
//! figure out if the library supports that hardware. To that end, the [DeviceConfigurationManager]
//! contains all of the APIs needed to load protocol configurations into the system, as well as
//! match newly discovered devices to protocols.
//!
//! ## Device Identification
//!
//! Once devices are connected, they are identified via the following properties:
//!
//! - Their communication bus address (BLE address, serial port name, etc... For devices that
//!   connect via network protocols, this may be a generated value, but should be unique.)
//! - Their protocol name
//! - Their protocol identifier
//!
//! These values are held in [ProtocolDeviceIdentifier] instances, and used around the codebase to
//! identify a device. This identifier is used so that if a device somehow shares addresses with
//! another device but identifies under a different protocol, they will still be seen as separate
//! devices.
//!
//! As an example, let's say we have a Lovense Hush. The protocol will be "lovense" (which is
//! configuration string version of the [Lovense Protocol](crate::device::protocol::lovense) name),
//! its identifier will be "Z" (the identification letter for Hush in Lovense's proprietary
//! protocol), and the address will be something like "AA:BB:CC:DD:EE:FF", which is the BLE address
//! of the device on platforms that provide BLE addresses. Using these 3 values means that, even if
//! for some reason the BLE address stays the same, if a device identifies differently (say, as a
//! Domi instead of a Hush), we won't try to reuse the same configuration.
//!
//! **NOTE THAT DEVICE IDENTIFIERS MAY NOT BE PORTABLE ACROSS PLATFORMS.** While these are used as
//! internal identifers as well as keys for user configurations, they may not work if used between,
//! say, Windows BLE and WebBluetooth, which provide different addressing schemes for devices.
//!
//! ## Device Configurations versus User Configurations
//!
//! Device Configurations are provided by the core Buttplug Team, and the configuration of all
//! currently supported devices is both compiled into the library as well as distributed as external
//! files (see the Device Configuration Files section below). However, users may want to set certain
//! per-device configurations, in which case, User Configurations can be used.
//!
//! User configurations include:
//!
//! - Device Allow/Deny Lists: library will either only connect to certain devices, or never connect
//!   to them, respectively.
//! - Reserved indexes: allows the same device to show up to clients on the same device index every
//!   time it connects
//! - Device configuration extensions: If a new device from a brand comes out and has not been added
//!   to the main Device Configuration file, or else a user creates their own DIY device that uses
//!   another protocol (hence it will never be in the main Device Configuration file as there may
//!   only be one of the device, period), a user can add an extension to an established protocol to
//!   provide new identifier information.
//! - User configured message attributes: limits that can be set for certain messages a device
//!   takes. For instance, setting an upper limit on the vibration speed of a vibrator so it will
//!   only go to 80% instead of 100%.
//!
//! User configurations can be added to the [DeviceConfigurationManager].
//!
//! ## Device Configuration Files
//!
//! While all device configuration can be created and handled via API calls, the library supports
//! 100s of devices, meaning doing this in code would be rather unwieldy, and any new device
//! additions would require library version revs. To allow for adding and updating configurations
//! possibly without the need for library updates, we externalize this configuration to JSON files.
//!
//! Similarly, GUIs and other utilities have been created to facilitate creation of User
//! Configurations, and these are also stored to files and loadable by the library.
//!
//! These files are handled in the [Device Configuration File Module in the Utils portion of the
//! library](crate::util::device_configuration). More information on the file format and loading
//! strategies can be found there.
//!
//! ## Architecture
//!
//! The [DeviceConfigurationManager] consists of a tree of types and usage flow that may be a bit
//! confusing, so we'll outline and summarize them here.
//!
//! At the top level is the [DeviceConfigurationManager] itself. It contains 4 different pieces of
//! information:
//!
//! - Protocol device specifiers and attributes
//! - Factory/Builder instances for [ButtplugProtocols](crate::device::protocol::ButtplugProtocol)
//! - User configuration information (allow/deny lists, per-device protocol attributes, etc...)
//!
//! The [DeviceConfigurationManager] is created when a ButtplugServer comes up, and which time
//! protocols and user configurations can be added. After this, it is queried any time a new device
//! is found, to see whether a registered protocol is usable with that device.
//!
//! ### Adding Protocols
//!
//! Adding protocols to the DCM happens via the add_protocol_factory and remove_protocol_factory
//! methods.
//!
//! ### Protocol Device Specifiers
//!
//! In order to know if a discovered device can be used by Buttplug, it needs to be checked for
//! identifying information. The library use "specifiers" (like [BluetoothLESpecifier],
//! [USBSpecifier], etc...) for this. Specifiers contain device identification and connection
//! information, and we compare groups of specifiers in protocol configurations (as part of the
//! [ProtocolDeviceConfiguration] instance) with a specifier built from discovered devices to see if
//! there are any matches.
//!
//! For instance, we know the Bluetooth LE information for WeVibe toys, all of which is stored with
//! the WeVibe protocol configuration. The WeVibe protocol configuration has a Bluetooth LE
//! specifier with all of that information. When someone has a, say, WeVibe Ditto, they can turn it
//! on and put it into bluetooth discovery mode. If Buttplug is scanning for devices, we'll see the
//! Ditto, via its corresponding Bluetooth advertisement. Data from this advertisement can be turned
//! into a Bluetooth LE specifier. We can then match the specifier made from the advertisement
//! against all the protocol specifiers in the system, and find that this device will work with the
//! WeVibe protocol, at which point we'll move to the next step, protocol building.
//!
//! ### Protocol Building
//!
//! If a discovered device matches one or more protocol specifiers, a connection attempt begins,
//! where each matched protocol is given a chance to see if it can identify and communicate with the
//! device. If a protocol and device are matched, and connection is successful the initialized
//! protocol instance is returned, and becomes part of the
//! [ButtplugDevice](crate::device::ButtplugDevice) instance used by the
//! [ButtplugServer](crate::server::ButtplugServer).
//!
//! ### User Configurations
//!

#[macro_use]
extern crate strum_macros;

#[macro_use]
extern crate log;

mod device_config_file;
pub use device_config_file::{load_protocol_configs, save_user_config};
mod device_config_manager;
pub use device_config_manager::*;
mod specifier;
pub use specifier::*;
mod identifiers;
pub use identifiers::*;
mod device_definitions;
pub use device_definitions::*;
mod device_feature;
pub use device_feature::*;
mod endpoint;
pub use endpoint::*;


use std::ops::RangeInclusive;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ButtplugDeviceConfigError<T> {
  /// Conversion to client type not possible with requested property type
  #[error("Conversion of {0} to client type not possible with requested property type")]
  InvalidOutputTypeConversion(String),
  /// User set range exceeds bounds of possible configuration range
  #[error("User set range {0} exceeds bounds of possible configuration range {1}")]
  InvalidUserRange(RangeInclusive<T>, RangeInclusive<T>),
  /// Base range required
  #[error("Base range required for all feature outputs")]
  BaseRangeRequired,
}
