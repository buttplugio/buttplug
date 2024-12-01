use super::v2::ClientDeviceMessageAttributesV2;
use super::v3::ClientDeviceMessageAttributesV3;
use crate::core::{errors::ButtplugError, message::DeviceFeature};
use getset::Getters;
use std::collections::HashMap;

#[derive(Debug, Getters, Clone)]
pub(crate) struct LegacyDeviceAttributes {
  /*  #[getset(get = "pub")]
  attrs_v1: ClientDeviceMessageAttributesV1,
  */
  #[getset(get = "pub")]
  attrs_v2: ClientDeviceMessageAttributesV2,
  #[getset(get = "pub")]
  attrs_v3: ClientDeviceMessageAttributesV3,
  #[getset(get = "pub")]
  features: Vec<DeviceFeature>,
}

impl LegacyDeviceAttributes {
  pub fn new(features: &Vec<DeviceFeature>) -> Self {
    Self {
      attrs_v3: ClientDeviceMessageAttributesV3::from(features.clone()),
      attrs_v2: ClientDeviceMessageAttributesV2::from(features.clone()),
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
