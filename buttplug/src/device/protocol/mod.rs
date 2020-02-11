pub mod aneros;
pub mod lovense;
pub mod maxpro;
pub mod picobong;
pub mod prettylove;
pub mod realov;
pub mod svakom;
pub mod youcups;
pub mod youou;
mod generic_command_manager;

use super::device::DeviceImpl;
use crate::
{
    core::
    {
        errors::ButtplugError,
        messages::
        {
            ButtplugDeviceCommandMessageUnion, ButtplugMessageUnion, MessageAttributesMap,
        },
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

// TODO These macros could use some compilation tests to make sure we're
// bringing in everything we need.

// Note: We have to use tt instead of ident here due to the async_trait macro.
// See https://github.com/dtolnay/async-trait/issues/46 for more info.
#[macro_export]
macro_rules! create_buttplug_protocol_impl (
    (
        $protocol_name:tt,
        $(
            ( $message_name:tt )
        ),+
    ) => {
        use async_trait::async_trait;
        use crate::{
            device::{
                Endpoint,
                device::DeviceWriteCmd,
                protocol::{ButtplugProtocol, ButtplugProtocolCreator},
                configuration_manager::DeviceProtocolConfiguration,
            },
            core::{
                messages::{
                    ButtplugMessageUnion,
                    ButtplugDeviceCommandMessageUnion,
                    $(
                        $message_name
                    ),*
                }
            },
        };

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

        paste::item! {
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
                                self.[<$message_name _handler>](device, msg).await
                            }
                        ),*
                        ButtplugDeviceCommandMessageUnion::StopDeviceCmd(msg) => {
                            self.handle_stop_device_cmd(device, msg).await
                        }
                        _ => Err(ButtplugError::ButtplugDeviceError(
                            ButtplugDeviceError::new("AnerosProtocol does not accept this message type."),
                        )),
                    }
                }
            }
        }
    }
);

#[macro_export]
macro_rules! create_buttplug_protocol (
    (
        $protocol_name:tt,
        (
            $( 
                ( $member_name:tt: $member_type:ty = $member_initial_value:expr )
            ),*
        ),
        (
            $(
                ( $message_name:tt, $message_handler_body:block )
            ),+
        )
    ) => {
        use crate::{
            create_buttplug_protocol_impl,
            device::{
                device::DeviceImpl,
                protocol::generic_command_manager::GenericCommandManager,
            },
            core::{
                errors::{ButtplugError, ButtplugDeviceError},
                messages::{self, ButtplugMessage, StopDeviceCmd, MessageAttributesMap},
            },
        };
        use async_std::sync::{Arc, Mutex};

        create_buttplug_protocol_impl!($protocol_name, $(
            ( $message_name )
        ),+);

        #[derive(Clone)]
        pub struct $protocol_name {
            name: String,
            attributes: MessageAttributesMap,
            manager: Arc<Mutex<GenericCommandManager>>,
            stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
            $(
                $member_name: $member_type
            ),*
        }

        paste::item! {
            impl $protocol_name {
                pub fn new(name: &str, attributes: MessageAttributesMap) -> Self {
                    let manager = GenericCommandManager::new(&attributes);

                    $protocol_name {
                        name: name.to_owned(),
                        attributes,
                        stop_commands: manager.get_stop_commands(),
                        manager: Arc::new(Mutex::new(manager)),
                        $(
                            $member_name: $member_initial_value
                        ),*
                    }
                }

                async fn handle_stop_device_cmd(
                    &mut self,
                    device: &Box<dyn DeviceImpl>,
                    stop_msg: &StopDeviceCmd,
                ) -> Result<ButtplugMessageUnion, ButtplugError> {
                    // TODO This clone definitely shouldn't be needed but I'm tired. GOOD FIRST BUG.
                    let cmds = self.stop_commands.clone();
                    for msg in cmds {
                        self.parse_message(device, &msg).await?;
                    }
                    Ok(messages::Ok::new(stop_msg.get_id()).into())
                }

                $(
                    #[allow(non_snake_case)]
                    pub async fn [<$message_name _handler>](&mut self,
                        device: &Box<dyn DeviceImpl>,
                        msg: &$message_name,) -> Result<ButtplugMessageUnion, ButtplugError>
                        $message_handler_body
                    )*
                }
            }
        }
    );
