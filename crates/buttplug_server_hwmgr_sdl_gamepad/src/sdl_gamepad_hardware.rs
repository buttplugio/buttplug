// Buttplug SDL2 Gamepad Hardware Implementation
//
// Wraps an SDL2 GameController as a buttplug device with vibrate support.
// write_value() receives left/right motor speeds as u16 LE and calls set_rumble().

use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server::device::hardware::{
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
};
use buttplug_server_device_config::{Endpoint, ProtocolCommunicationSpecifier, XInputSpecifier};
use byteorder::{LittleEndian, ReadBytesExt};
use futures::future::{self, BoxFuture, FutureExt};
use std::{
  fmt::{self, Debug},
  io::Cursor,
  sync::Arc,
};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

pub struct SdlGamepadHardwareConnector {
  joystick_index: u32,
  name: String,
}

impl SdlGamepadHardwareConnector {
  pub fn new(joystick_index: u32, name: String) -> Self {
    Self {
      joystick_index,
      name,
    }
  }
}

impl Debug for SdlGamepadHardwareConnector {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("SdlGamepadHardwareConnector")
      .field("joystick_index", &self.joystick_index)
      .field("name", &self.name)
      .finish()
  }
}

#[async_trait]
impl HardwareConnector for SdlGamepadHardwareConnector {
  fn specifier(&self) -> ProtocolCommunicationSpecifier {
    // Reuse XInput specifier — gamepad protocol is the same (left/right motor u16)
    ProtocolCommunicationSpecifier::XInput(XInputSpecifier::default())
  }

  async fn connect(&mut self) -> Result<Box<dyn HardwareSpecializer>, ButtplugDeviceError> {
    debug!("Creating SDL gamepad device for index {}", self.joystick_index);

    let hardware_internal = SdlGamepadHardware::new(self.joystick_index)?;
    let address = format!("sdl-gamepad-{}", self.joystick_index);

    let hardware = Hardware::new(
      &self.name,
      &address,
      &[Endpoint::Tx],
      &None,
      false,
      Box::new(hardware_internal),
    );

    Ok(Box::new(GenericHardwareSpecializer::new(hardware)))
  }
}

/// Holds SDL context and GameController handle.
/// SDL2 GameController is not Send, so we wrap in a thread-local approach
/// using a dedicated thread for SDL operations.
struct SdlWorker {
  /// Channel to send rumble commands to the SDL thread
  cmd_tx: std::sync::mpsc::Sender<SdlCommand>,
}

enum SdlCommand {
  Rumble {
    left: u16,
    right: u16,
    duration_ms: u32,
  },
  Stop,
  Quit,
}

impl SdlWorker {
  fn new(joystick_index: u32) -> Result<Self, ButtplugDeviceError> {
    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<SdlCommand>();

    // SDL must be used from a single thread. Spawn a dedicated thread.
    std::thread::spawn(move || {
      let sdl = match sdl2::init() {
        Ok(s) => s,
        Err(e) => {
          error!("SDL init failed in worker: {e}");
          return;
        }
      };
      let gc_subsystem = match sdl.game_controller() {
        Ok(gc) => gc,
        Err(e) => {
          error!("SDL GameController init failed: {e}");
          return;
        }
      };
      let mut controller = match gc_subsystem.open(joystick_index) {
        Ok(c) => c,
        Err(e) => {
          error!("Failed to open gamepad {joystick_index}: {e}");
          return;
        }
      };

      info!(
        "SDL Gamepad worker started for '{}' (index {}), rumble: {}",
        controller.name(),
        joystick_index,
        controller.has_rumble()
      );

      // Process commands until Quit.
      // Buttplug sends separate write_value calls for each motor, so we
      // drain all pending commands before applying rumble to avoid
      // intermediate states (e.g. left=65535,right=0 followed by left=0,right=0).
      loop {
        match cmd_rx.recv() {
          Ok(SdlCommand::Rumble {
            mut left,
            mut right,
            mut duration_ms,
          }) => {
            // Small delay to let both motor commands arrive before processing
            std::thread::sleep(std::time::Duration::from_millis(5));
            // Drain any additional pending commands
            while let Ok(next) = cmd_rx.try_recv() {
              match next {
                SdlCommand::Rumble { left: l, right: r, duration_ms: d } => {
                  left = l;
                  right = r;
                  duration_ms = d;
                }
                SdlCommand::Stop => {
                  left = 0;
                  right = 0;
                  duration_ms = 10000;
                }
                SdlCommand::Quit => {
                  let _ = controller.set_rumble(0, 0, 10000);
                  return;
                }
              }
            }
            if let Err(e) = controller.set_rumble(left, right, duration_ms) {
              warn!("SDL rumble failed: {e}");
            }
          }
          Ok(SdlCommand::Stop) => {
            let _ = controller.set_rumble(0, 0, 10000);
          }
          Ok(SdlCommand::Quit) | Err(_) => {
            let _ = controller.set_rumble(0, 0, 10000);
            break;
          }
        }
      }
      debug!("SDL Gamepad worker exiting for index {joystick_index}");
    });

    Ok(Self { cmd_tx })
  }

  fn rumble(&self, left: u16, right: u16, duration_ms: u32) {
    let _ = self.cmd_tx.send(SdlCommand::Rumble {
      left,
      right,
      duration_ms,
    });
  }

  fn stop(&self) {
    // Duration must be > 0 for SDL to actually process the rumble-off
    let _ = self.cmd_tx.send(SdlCommand::Rumble { left: 0, right: 0, duration_ms: 10 });
  }
}

impl Drop for SdlWorker {
  fn drop(&mut self) {
    let _ = self.cmd_tx.send(SdlCommand::Quit);
  }
}

pub struct SdlGamepadHardware {
  worker: Arc<SdlWorker>,
  event_sender: broadcast::Sender<HardwareEvent>,
  cancellation_token: CancellationToken,
}

impl SdlGamepadHardware {
  pub fn new(joystick_index: u32) -> Result<Self, ButtplugDeviceError> {
    let (event_sender, _) = broadcast::channel(256);
    let token = CancellationToken::new();

    let worker = SdlWorker::new(joystick_index)?;

    Ok(Self {
      worker: Arc::new(worker),
      event_sender,
      cancellation_token: token,
    })
  }
}

impl Clone for SdlGamepadHardware {
  fn clone(&self) -> Self {
    Self {
      worker: self.worker.clone(),
      event_sender: self.event_sender.clone(),
      cancellation_token: self.cancellation_token.clone(),
    }
  }
}

impl Debug for SdlGamepadHardware {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("SdlGamepadHardware").finish()
  }
}

impl HardwareInternal for SdlGamepadHardware {
  fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.event_sender.subscribe()
  }

  fn disconnect(&self) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    info!("SDL Gamepad: disconnect() called — stopping rumble");
    self.worker.rumble(0, 0, 10000);
    future::ready(Ok(())).boxed()
  }

  fn read_value(
    &self,
    _msg: &HardwareReadCmd,
  ) -> BoxFuture<'static, Result<HardwareReading, ButtplugDeviceError>> {
    // No battery reading support for SDL gamepads (yet)
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "SDL Gamepad does not support read".to_owned(),
    )))
    .boxed()
  }

  fn write_value(
    &self,
    msg: &HardwareWriteCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    // Data format: [left_motor_u16_le, right_motor_u16_le]
    // Same as XInput protocol
    let data = msg.data().clone();
    let worker = self.worker.clone();

    async move {
      let mut cursor = Cursor::new(data);
      let left_motor_speed = cursor
        .read_u16::<LittleEndian>()
        .expect("Packed in protocol, infallible");
      let right_motor_speed = cursor
        .read_u16::<LittleEndian>()
        .expect("Packed in protocol, infallible");

      info!("SDL Gamepad: write_value left={} right={}", left_motor_speed, right_motor_speed);

      // Always use long duration — SDL on macOS ignores short-duration zero rumble.
      worker.rumble(left_motor_speed, right_motor_speed, 10000);
      Ok(())
    }
    .boxed()
  }

  fn subscribe(
    &self,
    _msg: &HardwareSubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "SDL Gamepad does not support subscribe".to_owned(),
    )))
    .boxed()
  }

  fn unsubscribe(
    &self,
    _msg: &HardwareUnsubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "SDL Gamepad does not support unsubscribe".to_owned(),
    )))
    .boxed()
  }
}

impl Drop for SdlGamepadHardware {
  fn drop(&mut self) {
    self.cancellation_token.cancel();
  }
}
