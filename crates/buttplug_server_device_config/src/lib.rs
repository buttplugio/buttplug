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

mod specifier;
pub use specifier::*;
mod identifiers;
pub use identifiers::*;
mod device_definitions;
pub use device_definitions::*;
mod device_feature;
pub use device_feature::*;
mod device_configuration;
pub use device_configuration::*;
mod endpoint;
pub use endpoint::*;

use buttplug_core::errors::ButtplugDeviceError;
use dashmap::DashMap;
use getset::Getters;
use std::{
  collections::HashMap,
  fmt::{self, Debug},
};

#[macro_use]
extern crate log;

#[derive(Default, Clone)]
pub struct DeviceConfigurationManagerBuilder {
  communication_specifiers: HashMap<String, Vec<ProtocolCommunicationSpecifier>>,
  user_communication_specifiers: DashMap<String, Vec<ProtocolCommunicationSpecifier>>,
  base_device_definitions: HashMap<BaseDeviceIdentifier, BaseDeviceDefinition>,
  user_device_definitions: DashMap<UserDeviceIdentifier, DeviceDefinition>,
}

impl DeviceConfigurationManagerBuilder {
  pub fn communication_specifier(
    &mut self,
    protocol_name: &str,
    specifier: &[ProtocolCommunicationSpecifier],
  ) -> &mut Self {
    self
      .communication_specifiers
      .entry(protocol_name.to_owned())
      .or_default()
      .extend(specifier.iter().cloned());
    self
  }

  pub fn protocol_features(
    &mut self,
    identifier: &BaseDeviceIdentifier,
    features: &BaseDeviceDefinition,
  ) -> &mut Self {
    self
      .base_device_definitions
      .insert(identifier.clone(), features.clone());
    self
  }

  pub fn user_communication_specifier(
    &mut self,
    protocol_name: &str,
    specifier: &[ProtocolCommunicationSpecifier],
  ) -> &mut Self {
    self
      .user_communication_specifiers
      .entry(protocol_name.to_owned())
      .or_default()
      .extend(specifier.iter().cloned());
    self
  }

  pub fn user_protocol_features(
    &mut self,
    identifier: &UserDeviceIdentifier,
    features: &UserDeviceDefinition,
  ) -> &mut Self {
    if let Some((_, base_definition)) = self
      .base_device_definitions
      .iter()
      .find(|(_, x)| x.id() == features.base_id())
    {
      self.user_device_definitions.insert(
        identifier.clone(),
        DeviceDefinition::new(base_definition, features),
      );
    } else {
      error!(
        "Cannot find protocol with base id {} for user id {}",
        features.base_id(),
        features.id()
      )
    }
    self
  }

  pub fn finish(&mut self) -> Result<DeviceConfigurationManager, ButtplugDeviceError> {
    // Build and validate the protocol attributes tree.
    let mut attribute_tree_map = HashMap::new();

    // Add all the defaults first, they won't have parent attributes.
    for (ident, attr) in &self.base_device_definitions {
      /*
      for feature in attr.features() {
        if let Err(e) = feature.is_valid() {
          error!("Feature {attr:?} for ident {ident:?} is not valid, skipping addition: {e:?}");
          continue;
        }
      }
      */
      attribute_tree_map.insert(ident.clone(), attr.clone());
    }

    let user_attribute_tree_map = DashMap::new();
    // Finally, add in user configurations, which will have an address.
    for kv in &self.user_device_definitions {
      let (ident, attr) = (kv.key(), kv.value());
      for feature in attr.features() {
        if let Err(e) = feature.is_valid() {
          error!("Feature {attr:?} for ident {ident:?} is not valid, skipping addition: {e:?}");
          continue;
        }
      }
      user_attribute_tree_map.insert(kv.key().clone(), kv.value().clone());
    }

    Ok(DeviceConfigurationManager {
      base_communication_specifiers: self.communication_specifiers.clone(),
      user_communication_specifiers: self.user_communication_specifiers.clone(),
      base_device_definitions: attribute_tree_map,
      user_device_definitions: user_attribute_tree_map,
      //protocol_map,
    })
  }
}

/// Correlates information about protocols and which devices they support.
///
/// The [DeviceConfigurationManager] handles stores information about which device protocols the
/// library supports, as well as which devices can use those protocols. When a
/// [DeviceCommunicationManager](crate::server::device::communication_manager) finds a device during scanning,
/// device information is given to the [DeviceConfigurationManager] to decide whether Buttplug
/// should try to connect to and communicate with the device.
///
/// Assuming the device is supported by the library, the [DeviceConfigurationManager] also stores
/// information about what commands can be sent to the device (Vibrate, Rotate, etc...), and the
/// parameters for those commands (number of power levels, stroke distances, etc...).
#[derive(Getters)]
#[getset(get = "pub")]
pub struct DeviceConfigurationManager {
  /// Communication specifiers from the base device config, mapped from protocol name to vector of
  /// specifiers. Should not change/update during a session.
  base_communication_specifiers: HashMap<String, Vec<ProtocolCommunicationSpecifier>>,
  /// Device definitions from the base device config. Should not change/update during a session.
  base_device_definitions: HashMap<BaseDeviceIdentifier, BaseDeviceDefinition>,
  /// Communication specifiers provided by the user, mapped from protocol name to vector of
  /// specifiers. Loaded at session start, may change over life of session.
  user_communication_specifiers: DashMap<String, Vec<ProtocolCommunicationSpecifier>>,
  /// Device definitions from the user device config. Loaded at session start, may change over life
  /// of session.
  user_device_definitions: DashMap<UserDeviceIdentifier, DeviceDefinition>,
}

impl Debug for DeviceConfigurationManager {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("DeviceConfigurationManager").finish()
  }
}

impl Default for DeviceConfigurationManager {
  fn default() -> Self {
    // Unwrap allowed here because we assume our built in device config will
    // always work. System won't pass tests or possibly even build otherwise.
    DeviceConfigurationManagerBuilder::default()
      .finish()
      .expect("Default creation of a DCM should always work.")
  }
}

impl DeviceConfigurationManager {
  pub fn add_user_communication_specifier(
    &self,
    protocol: &str,
    specifier: &ProtocolCommunicationSpecifier,
  ) -> Result<(), ButtplugDeviceError> {
    //self.protocol_map.contains_key(protocol);
    self
      .user_communication_specifiers
      .entry(protocol.to_owned())
      .or_default()
      .push(specifier.clone());
    Ok(())
  }

  pub fn remove_user_communication_specifier(
    &self,
    protocol: &str,
    specifier: &ProtocolCommunicationSpecifier,
  ) {
    if let Some(mut specifiers) = self.user_communication_specifiers.get_mut(protocol) {
      let specifier_vec = specifiers.value_mut();
      *specifier_vec = specifier_vec
        .iter()
        .filter(|s| *specifier != **s)
        .cloned()
        .collect();
    }
  }

  pub fn add_user_device_definition(
    &self,
    identifier: &UserDeviceIdentifier,
    definition: &DeviceDefinition,
  ) -> Result<(), ButtplugDeviceError> {
    //self.protocol_map.contains_key(identifier.protocol());
    // Check validity of device
    let mut index = definition.user_config().index();
    let indexes: Vec<u32> = self.user_device_definitions().iter().map(|x| x.value().user_config().index()).collect();
    // If we just added 1 to the maximum value of the current indexes, someone decides to set an
    // index to u32::MAX-1, then we'd have a problem. This is kind of a shit solution but it'll work
    // quickly for anyone that's not actively fucking with us by manually playing with user config files.
    while indexes.contains(&index) {
      index = index.wrapping_add(1);
    }
    let mut def = definition.clone();
    *def.user_device_mut().user_config_mut().index_mut() = index;
    self
      .user_device_definitions
      .entry(identifier.clone())
      .insert(def);
    Ok(())
  }

  pub fn remove_user_device_definition(&self, identifier: &UserDeviceIdentifier) {
    self.user_device_definitions.remove(identifier);
  }

  pub fn address_allowed(&self, address: &str) -> bool {
    // Make sure the device isn't on the deny list
    if self
      .user_device_definitions
      .iter()
      .any(|kv| kv.key().address() == address && kv.value().user_config().deny())
    {
      // If device is outright denied, deny
      info!(
        "Device {} denied by configuration, not connecting.",
        address
      );
      false
    } else if self
      .user_device_definitions
      .iter()
      .any(|kv| kv.value().user_config().allow())
      && !self
        .user_device_definitions
        .iter()
        .any(|kv| kv.key().address() == address && kv.value().user_config().allow())
    {
      // If device is not on allow list and allow list isn't empty, deny
      info!(
        "Device {} not on allow list and allow list not empty, not connecting.",
        address
      );
      false
    } else {
      true
    }
  }

  fn device_index(&self, identifier: &UserDeviceIdentifier) -> u32 {
    // See if we have a reserved or reusable device index here.
    if let Some(config) = self.user_device_definitions.get(identifier) {
      let index = config.user_config().index();
      debug!("Found index {index} for device {identifier:?}");
      return index;
    }

    let current_indexes: Vec<u32> = self
      .user_device_definitions
      .iter()
      .map(|x| x.user_config().index())
      .collect();

    // Someone is gonna make a max device index in their config file just to fuck with me, therefore
    // we don't do "max + 1", we fill in holes (lol) in sequences. To whomever has 4 billion sex toys:
    // sorry your index finding for new devices is slow and takes 16GB of allocation every time we
    // want to search the index space.

    let mut index = 0;
    while current_indexes.contains(&index) {
      index += 1;
    }
    debug!("Generating and assigning index {index:?} for device {identifier:?}");
    index
  }

  /// Provides read-only access to the internal protocol/identifier map. Mainly
  /// used for WebBluetooth filter construction, but could also be handy for
  /// listing capabilities in UI, etc.
  pub fn protocol_device_configurations(
    &self,
  ) -> HashMap<String, Vec<ProtocolCommunicationSpecifier>> {
    self.base_communication_specifiers.clone()
  }

  pub fn device_definition(&self, identifier: &UserDeviceIdentifier) -> Option<DeviceDefinition> {
    let features = if let Some(attrs) = self.user_device_definitions.get(identifier) {
      debug!("User device config found for {:?}", identifier);
      attrs.clone()
    } else if let Some(attrs) = self.base_device_definitions.get(&BaseDeviceIdentifier::new(
      identifier.protocol(),
      identifier.identifier(),
    )) {
      debug!(
        "Protocol + Identifier device config found for {:?}",
        identifier
      );
      DeviceDefinition::new_from_base_definition(attrs, self.device_index(identifier))
    } else if let Some(attrs) = self
      .base_device_definitions
      .get(&BaseDeviceIdentifier::new(identifier.protocol(), &None))
    {
      debug!("Protocol device config found for {:?}", identifier);
      DeviceDefinition::new_from_base_definition(attrs, self.device_index(identifier))
    } else {
      return None;
    };

    // If this is a new device, it needs to be added to the user device definition map.
    //
    // Device definitions are looked up before we fully initialize a device, mostly for algorithm
    // preparation. There is a very small chance we may save the device config then error out when
    // we connect to the device, but we'll assume we may connect successfully later.
    if self.user_device_definitions.get(identifier).is_none() {
      self
        .user_device_definitions
        .insert(identifier.clone(), features.clone());
    }

    Some(features)
  }
}
