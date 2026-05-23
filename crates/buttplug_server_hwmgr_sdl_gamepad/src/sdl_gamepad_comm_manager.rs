// Buttplug SDL2 Gamepad Communication Manager
//
// Scans for connected game controllers via SDL2's GameController API.
// SDL2 types are !Send, so scanning runs on a dedicated thread.

use super::sdl_gamepad_hardware::SdlGamepadHardwareConnector;
use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server::device::hardware::communication::{
  HardwareCommunicationManager,
  HardwareCommunicationManagerBuilder,
  HardwareCommunicationManagerEvent,
  TimedRetryCommunicationManager,
  TimedRetryCommunicationManagerImpl,
};
use tokio::sync::mpsc;

#[derive(Default, Clone)]
pub struct SdlGamepadCommunicationManagerBuilder {}

impl HardwareCommunicationManagerBuilder for SdlGamepadCommunicationManagerBuilder {
  fn finish(
    &mut self,
    sender: mpsc::Sender<HardwareCommunicationManagerEvent>,
  ) -> Box<dyn HardwareCommunicationManager> {
    Box::new(TimedRetryCommunicationManager::new(
      SdlGamepadCommunicationManager::new(sender),
    ))
  }
}

pub struct SdlGamepadCommunicationManager {
  sender: mpsc::Sender<HardwareCommunicationManagerEvent>,
}

impl SdlGamepadCommunicationManager {
  fn new(sender: mpsc::Sender<HardwareCommunicationManagerEvent>) -> Self {
    Self { sender }
  }
}

/// Info about a discovered gamepad, sent from the SDL scan thread.
struct GamepadInfo {
  joystick_index: u32,
  name: String,
}

#[async_trait]
impl TimedRetryCommunicationManagerImpl for SdlGamepadCommunicationManager {
  fn name(&self) -> &'static str {
    "SdlGamepadCommunicationManager"
  }

  async fn scan(&self) -> Result<(), ButtplugDeviceError> {
    trace!("SDL Gamepad manager scanning for devices");

    // SDL types are !Send, so we scan on a dedicated std thread and send results back.
    let (tx, rx) = std::sync::mpsc::channel::<GamepadInfo>();

    std::thread::spawn(move || {
      let sdl = match sdl2::init() {
        Ok(s) => s,
        Err(e) => {
          error!("SDL init failed: {e}");
          return;
        }
      };
      let gc = match sdl.game_controller() {
        Ok(gc) => gc,
        Err(e) => {
          error!("SDL GameController init failed: {e}");
          return;
        }
      };
      let num = gc.num_joysticks().unwrap_or(0);
      for i in 0..num {
        if !gc.is_game_controller(i) {
          continue;
        }
        let name = gc.name_for_index(i).unwrap_or_else(|_| format!("Gamepad {i}"));
        let _ = tx.send(GamepadInfo {
          joystick_index: i,
          name,
        });
      }
    });

    // Collect results (thread exits quickly after enumeration)
    // Small delay to let the thread finish
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    while let Ok(info) = rx.try_recv() {
      let address = format!("sdl-gamepad-{}", info.joystick_index);
      debug!("SDL Gamepad found: {} (index {})", info.name, info.joystick_index);

      let device_creator = Box::new(SdlGamepadHardwareConnector::new(
        info.joystick_index,
        info.name.clone(),
      ));

      if self
        .sender
        .send(HardwareCommunicationManagerEvent::DeviceFound {
          name: info.name,
          address,
          creator: device_creator,
        })
        .await
        .is_err()
      {
        error!("Error sending device found from SDL Gamepad manager.");
        break;
      }
    }

    Ok(())
  }

  fn can_scan(&self) -> bool {
    true
  }
}
