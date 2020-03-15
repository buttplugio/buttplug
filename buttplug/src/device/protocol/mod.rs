mod generic_command_manager;

use super::device::DeviceImpl;
use crate::{
    core::{
        errors::ButtplugError,
        messages::{ButtplugDeviceCommandMessageUnion, ButtplugInMessage, ButtplugOutMessage, MessageAttributesMap},
    },
    device::configuration_manager::{DeviceProtocolConfiguration, ProtocolConstructor},
};
use async_trait::async_trait;
use std::collections::HashMap;

macro_rules! create_protocols(
    (
        $(
            ($protocol_config_name:tt, $protocol_module:tt, $protocol_name:tt)
        ),*
    ) => {
        paste::item! {
            $(
                mod $protocol_module;
                use $protocol_module::[<$protocol_name Creator>];
            )*

            pub fn create_protocol_creator_map() -> HashMap::<String, ProtocolConstructor> {
                // Do not try to use HashMap::new() here. We need the explicit typing,
                // otherwise we'll just get an anonymous closure type during insert that
                // won't match.
                let mut protocols = HashMap::<String, ProtocolConstructor>::new();

                $(
                    protocols.insert(
                        $protocol_config_name.to_owned(),
                        Box::new(|config: DeviceProtocolConfiguration| {
                            Box::new([<$protocol_name Creator>]::new(config))
                        }),
                    );
                )*
                protocols
            }
        }
    }
);

// IF YOU WANT TO ADD NEW PROTOCOLS TO THE SYSTEM, DO IT HERE.
//
// This takes a tuple per protocol:
//
// - the name of the protocol in the device configuration file
// - the name of the module
// - the base name of the protocol, as used in create_buttplug_protocol!
create_protocols!(
    ("aneros", aneros, Aneros),
    ("maxpro", maxpro, Maxpro),
    ("lovense", lovense, Lovense),
    ("picobong", picobong, Picobong),
    ("realov", realov, Realov),
    ("prettylove", prettylove, PrettyLove),
    ("svakom", svakom, Svakom),
    ("youcups", youcups, Youcups),
    ("youou", youou, Youou),
    ("lovehoney-desire", lovehoney_desire, LovehoneyDesire),
    ("vorze-sa", vorze_sa, VorzeSA)
);

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
    ) -> Result<ButtplugOutMessage, ButtplugError>;
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
macro_rules! create_protocol_creator_impl (
    (
        true,
        $protocol_name:tt
    ) => {
        use async_trait::async_trait;
        use crate::{
            device::{
                protocol::{ButtplugProtocol, ButtplugProtocolCreator},
                configuration_manager::DeviceProtocolConfiguration,
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
    };
    (
        false,
        $protocol_name:tt
    ) => {
    };
);

#[macro_export]
macro_rules! create_buttplug_protocol (
    (
        $protocol_name:tt,
        $create_protocol_creator_impl:tt,
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
            create_protocol_creator_impl,
            device::{
                Endpoint,
                device::{DeviceWriteCmd, DeviceImpl},
                protocol::generic_command_manager::GenericCommandManager,
            },
            core::{
                errors::{ButtplugError, ButtplugDeviceError},
                messages::{
                    self,
                    ButtplugMessage,
                    StopDeviceCmd,
                    MessageAttributesMap,
                    ButtplugOutMessage,
                    ButtplugDeviceCommandMessageUnion,
                    $(
                        $message_name
                    ),*
                },
            },
        };
        use async_std::sync::{Arc, Mutex};

        create_protocol_creator_impl!($create_protocol_creator_impl, $protocol_name);

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
                ) -> Result<ButtplugOutMessage, ButtplugError> {
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
                        msg: &$message_name,) -> Result<ButtplugOutMessage, ButtplugError>
                        $message_handler_body
                    )*
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
                    ) -> Result<ButtplugOutMessage, ButtplugError> {
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
