// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Communication manager for SDL3 gamepads.

use super::{
  sdl_gamepad_hardware::SdlGamepadHardwareConnector,
  sdl_task::{SdlGamepadBackend, SdlGamepadDesc, SdlTaskBackend, SdlTaskError},
};
use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server::device::hardware::communication::{
  HardwareCommunicationManager,
  HardwareCommunicationManagerBuilder,
  HardwareCommunicationManagerEvent,
  TimedRetryCommunicationManager,
  TimedRetryCommunicationManagerImpl,
};
use sdl3::joystick::JoystickId;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Creates a buttplug device address from an SDL3 instance ID. This is the
/// only place instance IDs become part of the buttplug address space.
pub(crate) fn create_address(id: JoystickId) -> String {
  format!("sdl-gamepad-{}", id.0)
}

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
  backend: Arc<dyn SdlGamepadBackend>,
}

impl SdlGamepadCommunicationManager {
  fn new(sender: mpsc::Sender<HardwareCommunicationManagerEvent>) -> Self {
    Self {
      sender,
      backend: Arc::new(SdlTaskBackend::global()),
    }
  }

  /// Real scan work: enumerate via the backend and emit one DeviceFound event
  /// per gamepad. Distinguishes transient enumeration failures from a dead
  /// event channel so [`scan`](TimedRetryCommunicationManagerImpl::scan) can
  /// swallow the former but stop the retry loop on the latter.
  async fn enumerate_or_fail(&self) -> Result<(), ScanFailure> {
    let gamepads: Vec<SdlGamepadDesc> = self
      .backend
      .gamepads()
      .await
      .map_err(|e: SdlTaskError| ScanFailure::Enumeration(device_error("scan", e)))?;
    for gamepad in gamepads {
      let address = create_address(gamepad.id);
      info!(
        "SDL gamepad manager found device {} at address {}",
        gamepad.name, address
      );
      if self
        .sender
        .send(HardwareCommunicationManagerEvent::DeviceFound {
          name: gamepad.name.clone(),
          address: address.clone(),
          creator: Box::new(SdlGamepadHardwareConnector::new(
            self.backend.clone(),
            gamepad.id,
            gamepad.name,
            address,
            gamepad.capabilities,
          )),
        })
        .await
        .is_err()
      {
        error!("Error sending device found message from SDL gamepad manager.");
        return Err(ScanFailure::EventChannelClosed);
      }
    }
    Ok(())
  }

  /// Error-propagating form. Production `scan` uses [`Self::enumerate_or_fail`]
  /// to distinguish failure classes; this form exists (and is exercised by
  /// tests) to assert the propagation contract: enumeration errors ARE
  /// propagated by the internal implementation and only swallowed at the
  /// trait boundary.
  #[cfg(test)]
  async fn enumerate_and_emit(&self) -> Result<(), ButtplugDeviceError> {
    self
      .enumerate_or_fail()
      .await
      .map_err(|failure| match failure {
        ScanFailure::Enumeration(e) => e,
        ScanFailure::EventChannelClosed => device_error("event send", SdlTaskError::ThreadClosed),
      })
  }
}

enum ScanFailure {
  Enumeration(ButtplugDeviceError),
  /// The event consumer is gone (server shutting down): permanent, the scan
  /// loop should stop instead of spinning forever.
  EventChannelClosed,
}

fn device_error(operation: &str, e: SdlTaskError) -> ButtplugDeviceError {
  ButtplugDeviceError::DeviceCommunicationError(format!(
    "SDL gamepad manager {operation} error: {e}"
  ))
}

#[async_trait]
impl TimedRetryCommunicationManagerImpl for SdlGamepadCommunicationManager {
  fn name(&self) -> &'static str {
    "SdlGamepadCommunicationManager"
  }

  async fn scan(&self) -> Result<(), ButtplugDeviceError> {
    trace!("SDL gamepad manager scanning for devices");
    // Transient enumeration failures are deliberately swallowed here with a
    // logged warning: TimedRetryCommunicationManager breaks its scan loop on
    // any Err while leaving scanning_status() true, so surfacing one would
    // silently kill discovery while still reporting "scanning". The retry
    // loop simply tries again on its next tick.
    //
    // A dead event channel is NOT transient (the consumer is gone), so that
    // failure is surfaced to deliberately stop the retry loop.
    match self.enumerate_or_fail().await {
      Ok(()) => {}
      Err(ScanFailure::Enumeration(e)) => {
        warn!("SDL gamepad manager scan failed, will retry: {e}");
      }
      Err(ScanFailure::EventChannelClosed) => {
        error!("SDL gamepad manager event channel closed; stopping scan loop.");
        return Err(device_error("event send", SdlTaskError::ThreadClosed));
      }
    }
    Ok(())
  }

  // If SDL failed to initialize at startup (published inert state), the
  // manager reports itself unable to scan.
  fn can_scan(&self) -> bool {
    self.backend.initialized()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::sdl_task::{SdlTaskError, joystick_id};
  use std::sync::Mutex as StdMutex;

  /// Mock outer-seam backend: configurable gamepad list / failure.
  struct MockBackend {
    gamepads: StdMutex<Result<Vec<SdlGamepadDesc>, SdlTaskError>>,
  }

  #[async_trait]
  impl SdlGamepadBackend for MockBackend {
    fn initialized(&self) -> bool {
      true
    }

    async fn gamepads(&self) -> Result<Vec<SdlGamepadDesc>, SdlTaskError> {
      self.gamepads.lock().unwrap().clone()
    }

    async fn open(
      &self,
      _id: JoystickId,
    ) -> Result<
      (
        Arc<dyn crate::sdl_task::SdlOpenedGamepad>,
        crate::sdl_task::SdlRumbleCapabilities,
      ),
      SdlTaskError,
    > {
      panic!("open is not exercised through this mock")
    }
  }

  fn manager_with(
    gamepads: Result<Vec<SdlGamepadDesc>, SdlTaskError>,
  ) -> (
    mpsc::Receiver<HardwareCommunicationManagerEvent>,
    SdlGamepadCommunicationManager,
  ) {
    let (tx, rx) = mpsc::channel(32);
    let manager = SdlGamepadCommunicationManager {
      sender: tx,
      backend: Arc::new(MockBackend {
        gamepads: StdMutex::new(gamepads),
      }),
    };
    (rx, manager)
  }

  fn desc(id: u32, name: &str) -> SdlGamepadDesc {
    SdlGamepadDesc {
      id: joystick_id(id),
      name: name.to_owned(),
      capabilities: crate::sdl_task::SdlRumbleCapabilities {
        rumble: true,
        trigger_rumble: false,
      },
    }
  }

  #[tokio::test]
  async fn comm_manager_scan_emits_device_found_with_stable_addresses() {
    let (mut rx, manager) = manager_with(Ok(vec![
      desc(3, "Xbox Wireless Controller"),
      desc(11, "DualSense Wireless Controller"),
    ]));

    manager.scan().await.expect("scan should succeed");

    let event = rx.recv().await.expect("first event");
    let HardwareCommunicationManagerEvent::DeviceFound { name, address, .. } = event else {
      panic!("expected DeviceFound, got {event:?}");
    };
    assert_eq!(name, "Xbox Wireless Controller");
    assert_eq!(address, "sdl-gamepad-3");

    let event = rx.recv().await.expect("second event");
    let HardwareCommunicationManagerEvent::DeviceFound { name, address, .. } = event else {
      panic!("expected DeviceFound, got {event:?}");
    };
    assert_eq!(name, "DualSense Wireless Controller");
    assert_eq!(address, "sdl-gamepad-11");

    // No further events: drop the manager so its event sender closes the
    // channel (recv only yields None once every sender is gone).
    drop(manager);
    assert!(rx.recv().await.is_none());
  }

  #[tokio::test]
  async fn comm_manager_scan_swallows_transient_enumeration_error() {
    let (mut rx, manager) = manager_with(Err(SdlTaskError::Scan("boom".to_owned())));

    // Trait-level scan returns Ok with no events (logged warn): a transient
    // failure must not break the timed-retry loop.
    manager.scan().await.expect("scan must swallow the error");

    // The internal enumerate_and_emit DOES propagate the error (the swallow
    // is only at the trait boundary).
    assert!(manager.enumerate_and_emit().await.is_err());

    // Drop the manager so the event channel closes before checking emptiness.
    drop(manager);
    assert!(rx.recv().await.is_none());

    // Recovery on the next scan emits devices; the retry loop stays intact.
    let (mut rx2, manager2) = manager_with(Ok(vec![desc(1, "SDL Gamepad 1")]));
    manager2.scan().await.expect("scan should succeed");
    let event = rx2.recv().await.expect("event after recovery");
    let HardwareCommunicationManagerEvent::DeviceFound { name, address, .. } = event else {
      panic!("expected DeviceFound, got {event:?}");
    };
    assert_eq!(name, "SDL Gamepad 1");
    assert_eq!(address, "sdl-gamepad-1");

    // A dead event channel (consumer gone) is permanent: scan surfaces Err so
    // the timed retry loop stops instead of spinning forever.
    drop(rx2);
    assert!(
      manager2.scan().await.is_err(),
      "scan must surface a dead event channel"
    );
  }
}
