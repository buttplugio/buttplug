
use buttplug_core::errors::ButtplugDeviceError;
use dashmap::DashMap;
use getset::Getters;
use std::{
  collections::HashMap,
  fmt::{self, Debug},
};

use crate::{BaseDeviceIdentifier, ProtocolCommunicationSpecifier, ServerDeviceDefinition, UserDeviceIdentifier};

#[derive(Default, Clone)]
pub struct DeviceConfigurationManagerBuilder {
  communication_specifiers: HashMap<String, Vec<ProtocolCommunicationSpecifier>>,
  user_communication_specifiers: DashMap<String, Vec<ProtocolCommunicationSpecifier>>,
  base_device_definitions: HashMap<BaseDeviceIdentifier, ServerDeviceDefinition>,
  user_device_definitions: DashMap<UserDeviceIdentifier, ServerDeviceDefinition>,
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
    features: &ServerDeviceDefinition,
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
    features: &ServerDeviceDefinition,
  ) -> &mut Self {
    if let Some((_, base_definition)) = self
      .base_device_definitions
      .iter()
      .find(|(_, x)| x.id() == features.base_id())
    {
      self.user_device_definitions.insert(
        identifier.clone(),
        ServerDeviceDefinition::new(base_definition, features),
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
  base_device_definitions: HashMap<BaseDeviceIdentifier, ServerDeviceDefinition>,
  /// Communication specifiers provided by the user, mapped from protocol name to vector of
  /// specifiers. Loaded at session start, may change over life of session.
  user_communication_specifiers: DashMap<String, Vec<ProtocolCommunicationSpecifier>>,
  /// Device definitions from the user device config. Loaded at session start, may change over life
  /// of session.
  user_device_definitions: DashMap<UserDeviceIdentifier, ServerDeviceDefinition>,
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
    definition: &ServerDeviceDefinition,
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
