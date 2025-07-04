pub mod svakom_alex;
pub mod svakom_alex_v2;
pub mod svakom_avaneo;
pub mod svakom_barnard;
pub mod svakom_barney;
pub mod svakom_dice;
pub mod svakom_dt250a;
pub mod svakom_iker;
pub mod svakom_jordan;
pub mod svakom_pulse;
pub mod svakom_sam;
pub mod svakom_sam2;
pub mod svakom_suitcase;
pub mod svakom_tarax;
pub mod svakom_v1;
pub mod svakom_v2;
pub mod svakom_v3;
pub mod svakom_v4;
pub mod svakom_v5;
pub mod svakom_v6;

use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::{
  DeviceDefinition,
  ProtocolCommunicationSpecifier,
  UserDeviceIdentifier,
};

use crate::device::{
  hardware::Hardware,
  protocol::{
    generic_protocol_initializer_setup,
    ProtocolHandler,
    ProtocolIdentifier,
    ProtocolInitializer,
  },
};
use async_trait::async_trait;
use std::sync::Arc;

generic_protocol_initializer_setup!(Svakom, "svakom");

#[derive(Default)]
pub struct SvakomInitializer {}

#[async_trait]
impl ProtocolInitializer for SvakomInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    def: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    if let Some(variant) = def.protocol_variant() {
      match variant.as_str() {
        "svakom_alex" => Ok(Arc::new(svakom_alex::SvakomAlex::default())),
        "svakom_alex_v2" => Ok(Arc::new(svakom_alex_v2::SvakomAlexV2::default())),
        //"svakom_avaneo" => Ok(Arc::new(svakom_avaneo::SvakomAvaNeo::default())),
        "svakom_barnard" => Ok(Arc::new(svakom_barnard::SvakomBarnard::default())),
        "svakom_barney" => Ok(Arc::new(svakom_barney::SvakomBarney::default())),
        "svakom_dice" => Ok(Arc::new(svakom_dice::SvakomDice::default())),
        //"svakom_dt250a" => svakom_dt250a::SvakomDT250AInitializer::default().initialize(hardware, def).await,
        "svakom_iker" => Ok(Arc::new(svakom_iker::SvakomIker::default())),
        "svakom_jordan" => Ok(Arc::new(svakom_jordan::SvakomJordan::default())),
        "svakom_pulse" => Ok(Arc::new(svakom_pulse::SvakomPulse::default())),
        "svakom_sam" => {
          svakom_sam::SvakomSamInitializer::default()
            .initialize(hardware, def)
            .await
        }
        "svakom_sam2" => Ok(Arc::new(svakom_sam2::SvakomSam2::default())),
        //"svakom_suitcase" => Ok(Arc::new(svakom_suitcase::SvakomSuitcase::default())),
        //"svakom_tarax" => Ok(Arc::new(svakom_tarax::SvakomTaraX::default())),
        "svakom_v1" => Ok(Arc::new(svakom_v1::SvakomV1::default())),
        "svakom_v2" => Ok(Arc::new(svakom_v2::SvakomV2::default())),
        "svakom_v3" => Ok(Arc::new(svakom_v3::SvakomV3::default())),
        "svakom_v4" => Ok(Arc::new(svakom_v4::SvakomV4::default())),
        "svakom_v5" => Ok(Arc::new(svakom_v5::SvakomV5::default())),
        "svakom_v6" => Ok(Arc::new(svakom_v6::SvakomV6::default())),
        _ => Err(ButtplugDeviceError::ProtocolNotImplemented(format!(
          "No protocol implementation for Vorze Device {}",
          hardware.name()
        ))),
      }
    } else {
      Err(ButtplugDeviceError::ProtocolNotImplemented(format!(
        "No protocol implementation for Vorze Device {}",
        hardware.name()
      )))
    }
  }
}
