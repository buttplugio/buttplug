use super::v2::ServerDeviceMessageAttributesV2;
use super::ServerDeviceMessageAttributesV3;
use buttplug_core::errors::ButtplugError;
use buttplug_server_device_config::ServerDeviceFeature;
use getset::Getters;
use std::collections::HashMap;

#[derive(Debug, Getters, Clone)]
pub(crate) struct ServerDeviceAttributes {
  /*  #[getset(get = "pub")]
  attrs_v1: ClientDeviceMessageAttributesV1,
  */
  #[getset(get = "pub")]
  attrs_v2: ServerDeviceMessageAttributesV2,
  #[getset(get = "pub")]
  attrs_v3: ServerDeviceMessageAttributesV3,
  #[getset(get = "pub")]
  features: Vec<ServerDeviceFeature>,
}

impl ServerDeviceAttributes {
  pub fn new(features: &Vec<ServerDeviceFeature>) -> Self {
    Self {
      attrs_v3: ServerDeviceMessageAttributesV3::from(features.clone()),
      attrs_v2: ServerDeviceMessageAttributesV2::from(features.clone()),
      /*
      attrs_v1: ClientDeviceMessageAttributesV1::from(features.clone()),
      */
      features: features.clone(),
    }
  }
}

pub(crate) trait TryFromClientMessage<T>
where
  Self: Sized,
{
  fn try_from_client_message(
    msg: T,
    features: &HashMap<u32, ServerDeviceAttributes>,
  ) -> Result<Self, ButtplugError>;
}
