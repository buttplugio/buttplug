pub mod aneros;
pub mod lovense;
pub mod maxpro;
pub mod picobong;
pub mod prettylove;
pub mod realov;
pub mod svakom;
pub mod youcups;
pub mod youou;

use super::device::DeviceImpl;
use crate::{
    core::{
        errors::ButtplugError,
        messages::{ButtplugDeviceCommandMessageUnion, ButtplugMessageUnion, MessageAttributesMap},
    }
};
use async_trait::async_trait;

#[async_trait]
pub trait ButtplugProtocolCreator: Sync + Send {
    async fn try_create_protocol(
        &self,
        device_impl: &Box<dyn DeviceImpl>,
    ) -> Result<Box<dyn ButtplugProtocol>, ButtplugError>;
}

// Note: We have to use tt instead of ident here due to the async_trait macro.
// See https://github.com/dtolnay/async-trait/issues/46 for more info.
#[macro_export]
macro_rules! create_buttplug_protocol_impl (
    (
        $protocol_name:tt,
        $(
            ( $message_name:tt, $message_handler:tt )
        ),+
    ) => {
        paste::item! {
            pub struct [<$protocol_name Creator>] {
                config: DeviceProtocolConfiguration,
            }

            impl [<$protocol_name Creator>] {
                pub fn new(config: DeviceProtocolConfiguration) -> Self {
                    Self { config }
                }
            }

            #[async_trait]
            impl ButtplugProtocolCreator for [<$protocol_name Creator>] {
                async fn try_create_protocol(
                    &self,
                    device_impl: &Box<dyn DeviceImpl>,
                ) -> Result<Box<dyn ButtplugProtocol>, ButtplugError> {
                    let (names, attrs) = self.config.get_attributes(device_impl.name()).unwrap();
                    let name = names.get("en-us").unwrap();
                    Ok(Box::new($protocol_name::new(name, attrs)))
                }
            }

        }

        #[async_trait]
        impl ButtplugProtocol for $protocol_name {
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
                    $(
                        ButtplugDeviceCommandMessageUnion::$message_name(msg) => {
                            self.$message_handler(device, msg).await
                        }
                    ),*
                        _ => Err(ButtplugError::ButtplugDeviceError(
                            ButtplugDeviceError::new("AnerosProtocol does not accept this message type."),
                        )),
                }
            }
        }
    }
);

#[async_trait]
pub trait ButtplugProtocol: Sync + Send {
    fn name(&self) -> &str;
    fn message_attributes(&self) -> MessageAttributesMap;
    fn box_clone(&self) -> Box<dyn ButtplugProtocol>;
    async fn parse_message(
        &mut self,
        device: &Box<dyn DeviceImpl>,
        message: &ButtplugDeviceCommandMessageUnion,
    ) -> Result<ButtplugMessageUnion, ButtplugError>;
}

impl Clone for Box<dyn ButtplugProtocol> {
    fn clone(&self) -> Box<dyn ButtplugProtocol> {
        self.box_clone()
    }
}
