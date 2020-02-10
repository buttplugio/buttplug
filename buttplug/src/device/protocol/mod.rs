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
use crate::
{
    core::
    {
        errors::{ButtplugError, ButtplugDeviceError},
        messages::
        {
            ButtplugDeviceCommandMessageUnion, ButtplugMessageUnion, MessageAttributesMap,
            RotateCmd, VibrateCmd, LinearCmd, VibrateSubcommand
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
        use async_trait::async_trait;
        use crate::{
            device::{
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

#[macro_export]
macro_rules! create_buttplug_protocol (
    (
        $protocol_name:tt,
        $(
            ( $message_name:tt, $message_handler:tt )
        ),+
    ) => {
        use crate::{
            create_buttplug_protocol_impl,
            device::protocol::GenericCommandManager,
            core::messages::MessageAttributesMap,
        };
        use async_std::sync::{Arc, Mutex};

        create_buttplug_protocol_impl!($protocol_name, $(
            ( $message_name, $message_handler )
        ),+);

        #[derive(Clone)]
        pub struct $protocol_name {
            name: String,
            attributes: MessageAttributesMap,
            manager: Arc<Mutex<GenericCommandManager>>,
        }

        impl $protocol_name {
            pub fn new(name: &str, attributes: MessageAttributesMap) -> Self {
                $protocol_name {
                    name: name.to_owned(),
                    // Borrow attributes before we store it.
                    manager: Arc::new(Mutex::new(GenericCommandManager::new(&attributes))),
                    attributes,
                }
            }
        }
    }
);

#[macro_export]
macro_rules! stop_device_cmd_vibration {
    () => {
        async fn handle_stop_device_cmd(
            &mut self,
            device: &Box<dyn DeviceImpl>,
            _: &StopDeviceCmd,
        ) -> Result<ButtplugMessageUnion, ButtplugError> {
            let msg = &self.manager.lock().await.create_vibration_stop_cmd();
            self.handle_vibrate_cmd(
                device,
                msg,
            )
            .await
        }
    };
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

pub struct GenericCommandManager {
    sent_vibration: bool,
    sent_rotation: bool,
    sent_linear: bool,
    vibrations: Vec<u32>,
    vibration_step_counts: Vec<u32>,
    rotations: Vec<(u32, bool)>,
    rotation_step_counts: Vec<u32>,
    linears: Vec<(u32, u32)>,
    linear_step_counts: Vec<u32>,
}

impl GenericCommandManager {
    pub fn new(attributes: &MessageAttributesMap) -> Self {
        let mut vibrations: Vec<u32> = vec![];
        let mut vibration_step_counts: Vec<u32> = vec![];
        let mut rotations: Vec<(u32, bool)> = vec![];
        let mut rotation_step_counts: Vec<u32> = vec![];
        let mut linears: Vec<(u32, u32)> = vec![];
        let mut linear_step_counts: Vec<u32> = vec![];

        if let Some(attr) = attributes.get("VibrateCmd") {
            if let Some(count) = attr.feature_count {
                vibrations = vec![0; count as usize];
            }
            if let Some(step_counts) = &attr.step_count {
                vibration_step_counts = step_counts.clone();
            }
        }
        if let Some(attr) = attributes.get("RotateCmd") {
            if let Some(count) = attr.feature_count {
                rotations = vec![(0, true); count as usize];
            }
            if let Some(step_counts) = &attr.step_count {
                rotation_step_counts = step_counts.clone();
            }
        }
        if let Some(attr) = attributes.get("LinearCmd") {
            if let Some(count) = attr.feature_count {
                linears = vec![(0, 0); count as usize];
            }
            if let Some(step_counts) = &attr.step_count {
                linear_step_counts = step_counts.clone();
            }
        }

        Self {
            sent_vibration: false,
            sent_rotation: false,
            sent_linear: false,
            vibrations,
            rotations,
            linears,
            vibration_step_counts,
            rotation_step_counts,
            linear_step_counts
        }
    }

    pub fn update_vibration(&mut self, msg: &VibrateCmd) -> Result<Vec<Option<u32>>, ButtplugError> {
        // First, make sure this is a valid command, that contains at least one
        // command.
        if msg.speeds.len() == 0 {
            return Err(ButtplugDeviceError::new(&format!("VibrateCmd has 0 commands, will not do anything.")).into());
        }

        // Now we convert from the generic 0.0-1.0 range to the StepCount
        // attribute given by the device config.

        // If we've already sent commands before, we should check against our
        // old values. Otherwise, we should always send whatever command we're
        // going to send.
        let mut result: Vec<Option<u32>> = vec![None; self.vibrations.len()];
        for speed_command in &msg.speeds {
            let index = speed_command.index as usize;
            // Since we're going to iterate here anyways, we do our index check
            // here instead of in a filter above.
            if index >= self.vibrations.len() {
                return Err(ButtplugDeviceError::new(&format!("VibrateCmd has {} commands, device has {} vibrators.",
                msg.speeds.len(), self.vibrations.len())).into());
            }

            let speed = (speed_command.speed * self.vibration_step_counts[index] as f64) as u32;
            // If we've already sent commands, we don't want to send them again,
            // because some of our communication busses are REALLY slow. Make sure
            // these values get None in our return vector.
            if !self.sent_vibration || speed != self.vibrations[index] {
                self.vibrations[index] = speed;
                result[index] = Some(speed);
            }
        }

        self.sent_vibration = true;

        // Return the command vector for the protocol to turn into proprietary commands
        Ok(result)
    }

    pub fn update_rotation(&mut self, msg: &RotateCmd) -> Result<Vec<Option<(u32, bool)>>, ButtplugError> {
        // First, make sure this is a valid command, that contains at least one
        // command.
        if msg.rotations.len() == 0 {
            return Err(ButtplugDeviceError::new(&format!("RotateCmd has 0 commands, will not do anything.")).into());
        }

        // Now we convert from the generic 0.0-1.0 range to the StepCount
        // attribute given by the device config.

        // If we've already sent commands before, we should check against our
        // old values. Otherwise, we should always send whatever command we're
        // going to send.
        let mut result: Vec<Option<(u32, bool)>> = vec![None; self.rotations.len()];
        for rotate_command in &msg.rotations {
            let index = rotate_command.index as usize;
            // Since we're going to iterate here anyways, we do our index check
            // here instead of in a filter above.
            if index >= self.rotations.len() {
                return Err(ButtplugDeviceError::new(&format!("RotateCmd has {} commands, device has {} rotators.",
                msg.rotations.len(), self.rotations.len())).into());
            }
            let speed = (rotate_command.speed * self.rotation_step_counts[index] as f64) as u32;
            let clockwise = rotate_command.clockwise;
            // If we've already sent commands, we don't want to send them again,
            // because some of our communication busses are REALLY slow. Make sure
            // these values get None in our return vector.
            if !self.sent_rotation || speed != self.rotations[index].0 || clockwise != self.rotations[index].1 {
                self.rotations[index] = (speed, clockwise);
                result[index] = Some((speed, clockwise));
            }
        }

        self.sent_rotation = true;

        // Return the command vector for the protocol to turn into proprietary commands
        Ok(result)
    }

    pub fn update_linear(&mut self, msg: &LinearCmd) -> Result<Option<Vec<(u32, u32)>>, ButtplugError> {
        // First, make sure this is a valid command, that doesn't contain an
        // index we can't reach.

        // If we've already sent commands before, we should check against our
        // old values. Otherwise, we should always send whatever command we're
        // going to send.

        // Now we convert from the generic 0.0-1.0 range to the StepCount
        // attribute given by the device config.

        // If we've already sent commands, we don't want to send them again,
        // because some of our communication busses are REALLY slow. Make sure
        // these values get None in our return vector.

        // Return the command vector for the protocol to turn into proprietary commands
        Ok(None)
    }

    pub fn create_vibration_stop_cmd(&self) -> VibrateCmd {
        // TODO There's gotta be a more concise way to do this.
        let mut subcommands = vec!();
        for i in 0..self.vibrations.len() {
            subcommands.push(VibrateSubcommand::new(i as u32, 0.0));
        }
        VibrateCmd::new(
            0,
            subcommands
        )
    }
}

#[cfg(test)]
mod test {

    use super::GenericCommandManager;
    use crate::core::{
        messages::{
            MessageAttributesMap, MessageAttributes, VibrateCmd, VibrateSubcommand, RotateCmd, RotationSubcommand
        }
    };
    #[test]
    pub fn test_command_generator_vibration() {
        let mut attributes_map = MessageAttributesMap::new();

        let mut vibrate_attributes = MessageAttributes::default();
        vibrate_attributes.feature_count = Some(2);
        vibrate_attributes.step_count = Some(vec![20, 20]);
        attributes_map.insert("VibrateCmd".to_owned(), vibrate_attributes);
        let mut mgr = GenericCommandManager::new(&attributes_map);
        let vibrate_msg = VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5), VibrateSubcommand::new(1, 0.5)]);
        assert_eq!(mgr.update_vibration(&vibrate_msg).unwrap(), vec![Some(10), Some(10)]);
        assert_eq!(mgr.update_vibration(&vibrate_msg).unwrap(), vec![None, None]);
        let vibrate_msg_2 = VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5), VibrateSubcommand::new(1, 0.75)]);
        assert_eq!(mgr.update_vibration(&vibrate_msg_2).unwrap(), vec![None, Some(15)]);
        let vibrate_msg_invalid = VibrateCmd::new(0, vec![VibrateSubcommand::new(2, 0.5)]);
        assert!(mgr.update_vibration(&vibrate_msg_invalid).is_err());
    }

    #[test]
    pub fn test_command_generator_rotation() {
        let mut attributes_map = MessageAttributesMap::new();

        let mut rotate_attributes = MessageAttributes::default();
        rotate_attributes.feature_count = Some(2);
        rotate_attributes.step_count = Some(vec![20, 20]);
        attributes_map.insert("RotateCmd".to_owned(), rotate_attributes);
        let mut mgr = GenericCommandManager::new(&attributes_map);
        let rotate_msg = RotateCmd::new(0, vec![RotationSubcommand::new(0, 0.5, true), RotationSubcommand::new(1, 0.5, true)]);
        assert_eq!(mgr.update_rotation(&rotate_msg).unwrap(), vec![Some((10, true)), Some((10, true))]);
        assert_eq!(mgr.update_rotation(&rotate_msg).unwrap(), vec![None, None]);
        let rotate_msg_2 = RotateCmd::new(0, vec![RotationSubcommand::new(0, 0.5, true), RotationSubcommand::new(1, 0.75, false)]);
        assert_eq!(mgr.update_rotation(&rotate_msg_2).unwrap(), vec![None, Some((15, false))]);
        let rotate_msg_invalid = RotateCmd::new(0, vec![RotationSubcommand::new(2, 0.5, true)]);
        assert!(mgr.update_rotation(&rotate_msg_invalid).is_err());
    }

    // TODO Write test for vibration stop generator
}
