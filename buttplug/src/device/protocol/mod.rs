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

pub struct GenericCommandManager {
    attributes: MessageAttributesMap,
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
            attributes: attributes.clone(),
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

    pub fn update_rotation(self, msg: &RotateCmd) -> Result<Option<Vec<(u32, bool)>>, ButtplugError> {
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

    pub fn update_linear(self, msg: &LinearCmd) -> Result<Option<Vec<(u32, u32)>>, ButtplugError> {
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
}

#[cfg(test)]
mod test {

    use super::GenericCommandManager;
    use crate::core::{
        messages::{
            MessageAttributesMap, MessageAttributes, VibrateCmd, VibrateSubcommand
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
}
