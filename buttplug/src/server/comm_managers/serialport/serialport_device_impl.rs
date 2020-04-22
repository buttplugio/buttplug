use crate::{
  device::{
      Endpoint,
      configuration_manager::{DeviceSpecifier, ProtocolDefinition},
      device::{ButtplugDeviceImplCreator, DeviceImpl, DeviceReadCmd, DeviceWriteCmd, DeviceSubscribeCmd, DeviceUnsubscribeCmd, BoundedDeviceEventBroadcaster},
  },
  core::{
      errors::{ButtplugError, ButtplugDeviceError},
      messages::RawReading,
  },
};
use async_trait::async_trait;


pub struct SerialPortDeviceImplCreator {

}

#[async_trait]
impl ButtplugDeviceImplCreator for SerialPortDeviceImplCreator {
  fn get_specifier(&self) -> DeviceSpecifier {
    unimplemented!();
  }

  async fn try_create_device_impl(
      &mut self,
      protocol: ProtocolDefinition,
  ) -> Result<Box<dyn DeviceImpl>, ButtplugError> {
      unimplemented!();
  }
}

pub struct SerialPortDeviceImpl {
  
}

#[async_trait]
impl DeviceImpl for SerialPortDeviceImplCreator {
  fn name(&self) -> &str {
    unimplemented!();
  }

  fn address(&self) -> &str {
    unimplemented!();
  }

  fn connected(&self) -> bool {
    unimplemented!();
  }

  fn endpoints(&self) -> Vec<Endpoint> {
    unimplemented!();
  }

  async fn disconnect(&mut self) {
    unimplemented!();
  }

  fn box_clone(&self) -> Box<dyn DeviceImpl> {
    unimplemented!();
  }

  fn get_event_receiver(&self) -> BoundedDeviceEventBroadcaster {
    unimplemented!();
  }

  async fn read_value(&self, msg: DeviceReadCmd) -> Result<RawReading, ButtplugError> {
    unimplemented!();
  }

  async fn write_value(&self, msg: DeviceWriteCmd) -> Result<(), ButtplugError> {
    unimplemented!();
  }

  async fn subscribe(&self, msg: DeviceSubscribeCmd) -> Result<(), ButtplugError> {
    unimplemented!();
  }

  async fn unsubscribe(&self, msg: DeviceUnsubscribeCmd) -> Result<(), ButtplugError> {
    unimplemented!();
  }
}
