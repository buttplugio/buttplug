use super::{ButtplugProtocol, ButtplugProtocolCreator};
use crate::{
    core::{
        errors::{ButtplugDeviceError, ButtplugError},
        messages::{
            self, ButtplugDeviceCommandMessageUnion, ButtplugMessageUnion, MessageAttributesMap,
            StopDeviceCmd,
        },
    },
    device::{configuration_manager::DeviceProtocolConfiguration, device::DeviceImpl},
};
use async_trait::async_trait;

pub struct LiboKegelProtocolCreator {
    config: DeviceProtocolConfiguration,
}

impl LiboKegelProtocolCreator {
    pub fn new(config: DeviceProtocolConfiguration) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ButtplugProtocolCreator for LiboKegelProtocolCreator {
    async fn try_create_protocol(
        &self,
        device_impl: &Box<dyn DeviceImpl>,
    ) -> Result<Box<dyn ButtplugProtocol>, ButtplugError> {
        let (names, attrs) = self.config.get_attributes(device_impl.name()).unwrap();
        let name = names.get("en-us").unwrap();
        Ok(Box::new(LiboKegelProtocol::new(name, attrs)))
    }
}

#[derive(Clone)]
pub struct LiboKegelProtocol {
    name: String,
    attributes: MessageAttributesMap,
}

impl LiboKegelProtocol {
    pub fn new(name: &str, attributes: MessageAttributesMap) -> Self {
        LiboKegelProtocol {
            name: name.to_owned(),
            attributes,
        }
    }
}

#[async_trait]
impl ButtplugProtocol for LiboKegelProtocol {
    fn name(&self) -> &str {
        &self.name
    }

    fn message_attributes(&self) -> MessageAttributesMap {
        self.attributes.clone()
    }

    fn box_clone(&self) -> Box<dyn ButtplugProtocol> {
        Box::new((*self).clone())
    }

    async fn parse_message(
        &mut self,
        device: &Box<dyn DeviceImpl>,
        message: &ButtplugDeviceCommandMessageUnion,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        match message {
            ButtplugDeviceCommandMessageUnion::StopDeviceCmd(msg) => {
                self.handle_stop_device_cmd(device, msg).await
            }
            _ => Err(ButtplugError::ButtplugDeviceError(
                ButtplugDeviceError::new("LiboKegelProtocol does not accept this message type."),
            )),
        }
    }
}

impl LiboKegelProtocol {
    async fn handle_stop_device_cmd(
        &self,
        _device: &Box<dyn DeviceImpl>,
        _: &StopDeviceCmd,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        Ok(ButtplugMessageUnion::Ok(messages::Ok::default()))
    }
}
