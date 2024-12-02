use super::ServerDeviceMessageAttributesV3;
use super::{server_device_feature::ServerDeviceFeature, v2::ServerDeviceMessageAttributesV2};
use crate::core::errors::ButtplugError;
use getset::Getters;
use std::collections::HashMap;

#[derive(Debug, Getters, Clone)]
pub(crate) struct LegacyDeviceAttributes {
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

impl LegacyDeviceAttributes {
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
    features: &HashMap<u32, LegacyDeviceAttributes>,
  ) -> Result<Self, ButtplugError>;
}
