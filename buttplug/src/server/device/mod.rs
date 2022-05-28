// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Device configuration, connection, and communication
//!
//! Welcome to the guts of Buttplug.
//!
//! Structs in the device module are used by the [Buttplug Server](crate::server) (specifically the
//! [Device Manager](crate::server::device_manager::DeviceManager)) to identify devices that
//! Buttplug can connect to, and match them to supported protocols in order to establish
//! communication, translate ButtplugMessages to raw hardware commands, and send those commands to
//! the hardware.
//!
//! # What even is a device in Buttplug?
//!
//! Devices in buttplug consist of two components:
//!
//! - Implementations (represented by [Hardware]), which handle the actual communication with
//!   hardware. Implementations are created by a [DeviceCommunicationManager], which handles the
//!   discovery method for that type of hardware (Bluetooth scanning, USB bus scanning, listening on
//!   network ports, etc...)
//! - Protocols (represented by [ButtplugProtocol]), which hold information about the capabilities
//!   of a device (can it vibrate/rotate/etc, at what speeds, so on and so forth), and translate
//!   from [Buttplug Device Messages](crate::core::messages::ButtplugDeviceMessage) into strings or
//!   binary arrays to send to devices via their implementation.
//!
//! # Device Lifetimes in Buttplug
//!
//! Creation and handling of devices happens in stages in Buttplug: configuring what the library can
//! support, creating a device once a usable one is found, commanding connected devices, and
//! disconnection/reconnection. This is process makes up most of the reason the library exists, so
//! we'll cover it at a high level here.
//!
//! ## Configuration and Bringup
//!
//! Configuration of the device creation system happens when we bring up a
//! [ButtplugServer](crate::server::ButtplugServer) and configure the [DeviceManager] that is owns.
//! Information that needs to be added for device creation includes:
//!
//! - Protocols that the library implements, or that developers add themselves.
//! - Device configurations related to those protocols, so we can identify and connect to devices
//!   that are compatible with them.
//! - Lists of device addresses that we will either never connect to or only connect to.
//!
//! This information is entered via the public [DeviceManager] API, and stored between the
//! [DeviceManager] and the [DeviceConfigurationManager] (which is owned by the [DeviceManager]).
//!
//! After all of the information is added, the [DeviceManager] is considered ready to discover
//! devices.
//!
//! ## Device Discovery and Creation
//!
//! To create a device, we go through the following steps:
//!
//! - When the server receives a StartScanning message, all comm managers start looking for devices.
//!   Strategies for scanning can vary between [DeviceCommunicationManager]s, either using long term
//!   scans (bluetooth) or repeated timed scans (USB, HID, XInput, etc... which check their
//!   respective busses once per second) for new devices.
//! - For each device that is found in any [DeviceCommunicationManager], we emit a DeviceFound event
//!   with that device's identifying information. This information is sent to the
//!   [DeviceConfigurationManager], in order to make sure we can connect (we won't try to connect to
//!   devices we're already connected to, or to devices on the deny list, or to any device that
//!   /isn't/ on the allow list if it exists) identify protocols that may work with the device. With
//!   some protocols and types of communication, we require being connected to the device to discern
//!   the specific protocol to use, so this step may return more than one potential protocol. For
//!   instance, if we're working with Bluetooth LE advertisements, we only get a certain set of info
//!   in ads and may need to have the device connected to figure out exactly what type of device it
//!   is and what it supports.
//!   - If not matching protocols are found, we end discovery of the device at this step. If the
//!     advertisement information changes (i.e. if we get more bluetooth advertisement information),
//!     we may try connecting to the device again. For devices where the info won't change, we
//!     ignore the device until a new scan session is started.
//! - If matching protocols are returned, we move on to the device connection and setup phase. We
//!   first establish a hardware connection with the device, and then cycle through each protocol we
//!   were handed to see if any of them work with the device. We accept the first protocol found
//!   that works with the device.
//!   - If none of the protocols we were found match, we disconnect and end discovery.
//! - Once we have both connected hardware and a protocol, we run the protocol's initialization
//!   step. For some protocols, this is required to know what type of device we're talking to or to
//!   put the device in a mode where we can interact with it. However, for many protocols, this is
//!   just a no-op and the device connects ready to run.
//! - Finally, with everything connected and configured, we have all the information we need to see
//!   if there are any user configurations to apply to the device. This is where users can set
//!   limits different aspects of specific devices, like vibration speed, stroke length, etc...
//!
//! Once we've made it through this, the device is handed to the [DeviceManager], and the
//! [ButtplugServer] notifies the [ButtplugClient] (if one is connected) of the new device via the
//! DeviceAdded message.
//!
//! ## Commanding
//!
//!

pub mod configuration;
pub mod hardware;
pub mod protocol;
pub mod server_device;
mod server_device_manager;
mod server_device_manager_event_loop;

pub use server_device::{ServerDevice, ServerDeviceEvent, ServerDeviceIdentifier};
pub use server_device_manager::{ServerDeviceManager, ServerDeviceManagerBuilder};
