use crate::server::device::hardware::communication::HardwareSpecificError;
use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    configuration::{ProtocolCommunicationSpecifier, SDL2Specifier},
    hardware::{
      communication::sdl2::sdl2_device_comm_manager::SDL2JoystickActorHandle,
      GenericHardwareSpecializer,
      Hardware,
      HardwareConnector,
      HardwareEvent,
      HardwareInternal,
      HardwareReadCmd,
      HardwareReading,
      HardwareSpecializer,
      HardwareSubscribeCmd,
      HardwareUnsubscribeCmd,
      HardwareWriteCmd,
    },
  },
};
use async_trait::async_trait;
use byteorder::{LittleEndian, ReadBytesExt};
use futures::future::{self, BoxFuture, FutureExt};
use sdl2::joystick::PowerLevel;
use std::io::Cursor;
use tokio::sync::broadcast;

#[derive(Debug)]
struct SDL2HardwareConnectArgs {
  name: String,
  address: String,
  joystick: SDL2JoystickActorHandle,
  event_sender: broadcast::Sender<HardwareEvent>,
}

#[derive(Debug)]
pub struct SDL2HardwareConnector {
  args: Option<SDL2HardwareConnectArgs>,
}

impl SDL2HardwareConnector {
  pub fn new(
    name: String,
    address: String,
    joystick: SDL2JoystickActorHandle,
    event_sender: broadcast::Sender<HardwareEvent>,
  ) -> Self {
    Self {
      args: Some(SDL2HardwareConnectArgs {
        name,
        address,
        joystick,
        event_sender,
      }),
    }
  }
}

#[async_trait]
impl HardwareConnector for SDL2HardwareConnector {
  fn specifier(&self) -> ProtocolCommunicationSpecifier {
    ProtocolCommunicationSpecifier::SDL2(SDL2Specifier::default())
  }

  async fn connect(&mut self) -> Result<Box<dyn HardwareSpecializer>, ButtplugDeviceError> {
    if let Some(args) = self.args.take() {
      debug!(
        "SDL2 connector emitting a new SDL2 device impl: {name}, {address}",
        name = args.name,
        address = args.address
      );
      let hardware_internal = SDL2Hardware::new(args.joystick, args.event_sender);
      let hardware = Hardware::new(
        &args.name,
        &args.address,
        &[Endpoint::TxVibrate, Endpoint::RxBLEBattery],
        Box::new(hardware_internal),
      );
      Ok(Box::new(GenericHardwareSpecializer::new(hardware)))
    } else {
      Err(ButtplugDeviceError::DeviceSpecificError(
        HardwareSpecificError::SDL2Error(
          "SDL2 hardware connectors shouldn't be reused!".to_owned(),
        ),
      ))
    }
  }
}

pub struct SDL2Hardware {
  joystick: SDL2JoystickActorHandle,
  event_sender: broadcast::Sender<HardwareEvent>,
}

impl SDL2Hardware {
  fn new(
    joystick: SDL2JoystickActorHandle,
    event_sender: broadcast::Sender<HardwareEvent>,
  ) -> Self {
    Self {
      joystick,
      event_sender,
    }
  }
}

impl HardwareInternal for SDL2Hardware {
  /// We shouldn't have to do anything, assuming the `SDL2Hardware` gets dropped sometime after this,
  /// which should close the `Joystick` and its underlying SDL2 object.
  fn disconnect(&self) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Ok(())).boxed()
  }

  fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.event_sender.subscribe()
  }

  fn read_value(
    &self,
    msg: &HardwareReadCmd,
  ) -> BoxFuture<'static, Result<HardwareReading, ButtplugDeviceError>> {
    if msg.endpoint != Endpoint::RxBLEBattery {
      return future::ready(Err(ButtplugDeviceError::InvalidEndpoint(msg.endpoint))).boxed();
    }
    let joystick = self.joystick.clone();
    async move {
      match joystick.power_level().await {
        Ok(r) => match r {
          PowerLevel::Unknown => Err(ButtplugDeviceError::DeviceSpecificError(
            HardwareSpecificError::SDL2Error(
              "SDL2 couldn't read joystick battery level".to_owned(),
            ),
          )),
          PowerLevel::Empty => Ok(0),
          PowerLevel::Low => Ok(33),
          PowerLevel::Medium => Ok(66),
          PowerLevel::Full => Ok(100),
          PowerLevel::Wired => Ok(100),
        },
        Err(e) => Err(ButtplugDeviceError::DeviceSpecificError(
          HardwareSpecificError::SDL2Error(e),
        )),
      }
      .map(|r| HardwareReading::new(Endpoint::Rx, &vec![r]))
    }
    .boxed()
  }

  fn write_value(
    &self,
    msg: &HardwareWriteCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    if msg.endpoint != Endpoint::TxVibrate {
      return future::ready(Err(ButtplugDeviceError::InvalidEndpoint(msg.endpoint))).boxed();
    }
    let mut cursor = Cursor::new(msg.data.clone());
    let low_frequency_rumble = cursor
      .read_u16::<LittleEndian>()
      .expect("Packed in protocol, infallible");
    let high_frequency_rumble = cursor
      .read_u16::<LittleEndian>()
      .expect("Packed in protocol, infallible");
    let joystick = self.joystick.clone();
    async move {
      joystick
        .rumble(
          low_frequency_rumble,
          high_frequency_rumble,
          0, // indefinitely
        )
        .await
        .map_err(|e| ButtplugDeviceError::DeviceSpecificError(HardwareSpecificError::SDL2Error(e)))
    }
    .boxed()
  }

  fn subscribe(
    &self,
    _msg: &HardwareSubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "SDL2 hardware does not support subscribe".to_owned(),
    )))
    .boxed()
  }

  fn unsubscribe(
    &self,
    _msg: &HardwareUnsubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "SDL2 hardware does not support unsubscribe".to_owned(),
    )))
    .boxed()
  }
}
