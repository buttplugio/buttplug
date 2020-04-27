use super::{ButtplugProtocol, ButtplugProtocolCommandHandler, ButtplugProtocolCreator};
use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::{ButtplugDeviceCommandMessageUnion, MessageAttributesMap},
  },
  device::{
    configuration_manager::DeviceProtocolConfiguration,
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    ButtplugDeviceEvent, DeviceImpl, DeviceSubscribeCmd, DeviceUnsubscribeCmd,
    Endpoint,
  },
};
use futures::{future::BoxFuture, StreamExt};

#[derive(ButtplugProtocol, ButtplugProtocolProperties)]
pub struct TCode {
  name: String,
  message_attributes: MessageAttributesMap,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl TCode {
  pub(super) fn new(name: &str, message_attributes: MessageAttributesMap) -> Self {
    let manager = GenericCommandManager::new(&message_attributes);

    Self {
      name: name.to_owned(),
      message_attributes,
      stop_commands: manager.get_stop_commands(),
    }
  }
}

impl ButtplugProtocolCreator for TCode {
  fn new_protocol(name: &str, attrs: MessageAttributesMap) -> Box<dyn ButtplugProtocol> {
    Box::new(Self::new(name, attrs))
  }

  fn try_create(
    device_impl: &dyn DeviceImpl,
    configuration: DeviceProtocolConfiguration,
  ) -> BoxFuture<'static, Result<Box<dyn ButtplugProtocol>, ButtplugError>> {
    let subscribe_fut = device_impl.subscribe(DeviceSubscribeCmd::new(Endpoint::Rx));
    let mut event_receiver = device_impl.get_event_receiver();
    let unsubscribe_fut = device_impl.unsubscribe(DeviceUnsubscribeCmd::new(Endpoint::Rx));
    Box::pin(async move {
      info!("Trying to do TCode bringup!");
      subscribe_fut.await?;
      let identifier;
      match event_receiver.next().await {
        Some(ButtplugDeviceEvent::Notification(_, n)) => {
          let type_response = std::str::from_utf8(&n).unwrap().to_owned();
          info!("TCode Response: {}", type_response);
          identifier = "/dev/ttyACM0";
        }
        Some(ButtplugDeviceEvent::Removed) => {
          return Err(
            ButtplugDeviceError::ProtocolSpecificError(
              "TCode",
              "TCode Device disconnected while getting DeviceType info.",
            )
            .into(),
          );
        }
        None => {
          return Err(
            ButtplugDeviceError::ProtocolSpecificError("TCode", "Did not get TCode output in time")
              .into(),
          );
        }
      };
      unsubscribe_fut.await?;

      let (names, attrs) = configuration.get_attributes(&identifier).unwrap();
      let name = names.get("en-us").unwrap();
      Ok(Self::new_protocol(name, attrs))
    })
  }
}

impl ButtplugProtocolCommandHandler for TCode {}
