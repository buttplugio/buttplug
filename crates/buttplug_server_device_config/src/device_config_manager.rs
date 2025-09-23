use buttplug_core::errors::ButtplugDeviceError;
use dashmap::DashMap;
use getset::Getters;
use std::{
  collections::HashMap,
  fmt::{self, Debug},
};
use uuid::Uuid;

use crate::{
  BaseDeviceIdentifier,
  ButtplugDeviceConfigError,
  ProtocolCommunicationSpecifier,
  ServerDeviceDefinition,
  ServerDeviceDefinitionBuilder,
  UserDeviceIdentifier,
};

#[derive(Default, Clone)]
pub struct DeviceConfigurationManagerBuilder {
  base_communication_specifiers: HashMap<String, Vec<ProtocolCommunicationSpecifier>>,
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
      .base_communication_specifiers
      .entry(protocol_name.to_owned())
      .or_default()
      .extend(specifier.iter().cloned());
    self
  }

  pub fn base_device_definition(
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

  pub fn user_device_definition(
    &mut self,
    identifier: &UserDeviceIdentifier,
    device_definition: &ServerDeviceDefinition,
  ) -> Result<&mut Self, ButtplugDeviceConfigError> {
    if self
      .base_device_definitions
      .iter()
      .any(|(_, x)| x.id() == device_definition.base_id().unwrap_or_default())
    {
      self
        .user_device_definitions
        .insert(identifier.clone(), device_definition.clone());
      Ok(self)
    } else {
      error!(
        "Cannot find protocol with base id {:?} for user id {}",
        device_definition.base_id(),
        device_definition.id()
      );
      Err(ButtplugDeviceConfigError::BaseIdNotFound(
        device_definition.id(),
      ))
    }
  }

  pub fn finish(&mut self) -> Result<DeviceConfigurationManager, ButtplugDeviceError> {
    // Build and validate the protocol attributes tree.
    let mut attribute_tree_map = HashMap::new();

    // Add all the defaults first, they won't have parent attributes.
    for (ident, attr) in &self.base_device_definitions {
      attribute_tree_map.insert(ident.clone(), attr.clone());
    }

    let user_attribute_tree_map = DashMap::new();
    // Finally, add in user configurations, which will have an address.
    for kv in &self.user_device_definitions {
      user_attribute_tree_map.insert(kv.key().clone(), kv.value().clone());
    }

    Ok(DeviceConfigurationManager {
      base_communication_specifiers: self.base_communication_specifiers.clone(),
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
#[getset(get = "pub(crate)")]
pub struct DeviceConfigurationManager {
  /// Communication specifiers from the base device config, mapped from protocol name to vector of
  /// specifiers. Should not change/update during a session.
  #[getset(get = "pub")]
  base_communication_specifiers: HashMap<String, Vec<ProtocolCommunicationSpecifier>>,
  /// Device definitions from the base device config. Should not change/update during a session.
  base_device_definitions: HashMap<BaseDeviceIdentifier, ServerDeviceDefinition>,
  /// Communication specifiers provided by the user, mapped from protocol name to vector of
  /// specifiers. Loaded at session start, may change over life of session.
  #[getset(get = "pub")]
  user_communication_specifiers: DashMap<String, Vec<ProtocolCommunicationSpecifier>>,
  /// Device definitions from the user device config. Loaded at session start, may change over life
  /// of session.
  #[getset(get = "pub")]
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

  pub fn add_user_device_definition(&self, identifier: &UserDeviceIdentifier, definition: &ServerDeviceDefinition) {
    // TODO we should actually check validity of the definition we're adding here, i.e. does it have
    // a base id, is that ID in our base selections, etc...
    self.user_device_definitions.insert(identifier.clone(), definition.clone());
  }

  pub fn remove_user_device_definition(&self, identifier: &UserDeviceIdentifier) {
    self.user_device_definitions.remove(identifier);
  }

  pub fn address_allowed(&self, address: &str) -> bool {
    // Make sure the device isn't on the deny list
    if self
      .user_device_definitions
      .iter()
      .any(|kv| kv.key().address() == address && kv.value().deny())
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
      .any(|kv| kv.value().allow())
      && !self
        .user_device_definitions
        .iter()
        .any(|kv| kv.key().address() == address && kv.value().allow())
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
      let index = config.index();
      debug!("Found index {index} for device {identifier:?}");
      return index;
    }

    let current_indexes: Vec<u32> = self
      .user_device_definitions
      .iter()
      .map(|x| x.index())
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

  pub fn device_definition(
    &self,
    identifier: &UserDeviceIdentifier,
  ) -> Option<ServerDeviceDefinition> {
    let features = if let Some(definition) = self.user_device_definitions.get(identifier) {
      debug!("User device config found for {:?}", identifier);
      definition.clone()
    } else if let Some(definition) = self.base_device_definitions.get(&BaseDeviceIdentifier::new(
      identifier.protocol(),
      identifier.identifier(),
    )) {
      debug!(
        "Protocol + Identifier device config found for {:?}, creating new user device from configuration",
        identifier
      );
      let mut builder = ServerDeviceDefinitionBuilder::from_base(definition, Uuid::new_v4(), true);
      builder.index(self.device_index(identifier)).finish()
    } else if let Some(definition) = self
      .base_device_definitions
      .get(&BaseDeviceIdentifier::new(identifier.protocol(), &None))
    {
      debug!(
        "Protocol device config found for {:?}, creating new user device from protocol defaults",
        identifier
      );
      let mut builder = ServerDeviceDefinitionBuilder::from_base(definition, Uuid::new_v4(), true);
      builder.index(self.device_index(identifier)).finish()
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
