// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::ButtplugResultFuture,
  server::device::hardware::{
    communication::{
      sdl2::sdl2_hardware::{SDL2HardwareConnector, SDL2JoystickActor},
      HardwareCommunicationManager,
      HardwareCommunicationManagerBuilder,
      HardwareCommunicationManagerEvent,
    },
    HardwareEvent,
  },
};
use async_trait::async_trait;
use futures_util::FutureExt;
use sdl2::{self, event::Event, EventPump, JoystickSubsystem};
use std::{
  collections::HashMap,
  future,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  thread,
};
use tokio::{
  sync::{broadcast, mpsc},
  task::{self, LocalSet},
};

#[derive(Default, Clone)]
pub struct SDL2DeviceCommunicationManagerBuilder {}

impl HardwareCommunicationManagerBuilder for SDL2DeviceCommunicationManagerBuilder {
  fn finish(
    &mut self,
    sender: mpsc::Sender<HardwareCommunicationManagerEvent>,
  ) -> Box<dyn HardwareCommunicationManager> {
    Box::new(SDL2DeviceCommunicationManager::new(sender))
  }
}

pub struct SDL2DeviceCommunicationManager {
  scanning_status: Arc<AtomicBool>,
}

impl SDL2DeviceCommunicationManager {
  fn new(sender: mpsc::Sender<HardwareCommunicationManagerEvent>) -> Self {
    let scanning_status = Arc::new(AtomicBool::new(false));

    {
      let scanning_status = scanning_status.clone();
      thread::Builder::new()
        .name("sdl2-event-loop-thread".to_owned())
        .spawn(move || {
          if let Err(e) = sdl2_event_loop_thread(sender, scanning_status) {
            error!("SDL2 comm manager: {e}");
          }
        })
        .expect("Couldn't spawn SDL event loop thread!")
    };

    Self { scanning_status }
  }
}

// We're always watching for SDL controller connection/disconnection events.
// The scan status controls whether we report them as comm manager events.
#[async_trait]
impl HardwareCommunicationManager for SDL2DeviceCommunicationManager {
  fn name(&self) -> &'static str {
    "SDL2DeviceCommunicationManager"
  }

  fn start_scanning(&mut self) -> ButtplugResultFuture {
    trace!("SDL2 manager starting scan");
    self.scanning_status.store(true, Ordering::SeqCst);
    future::ready(Ok(())).boxed()
  }

  fn stop_scanning(&mut self) -> ButtplugResultFuture {
    trace!("SDL2 manager stopping scan");
    self.scanning_status.store(true, Ordering::SeqCst);
    future::ready(Ok(())).boxed()
  }

  fn scanning_status(&self) -> bool {
    self.scanning_status.load(Ordering::SeqCst)
  }

  fn can_scan(&self) -> bool {
    true
  }
}

/// Only one thread is allowed to talk to the SDL event loop,
/// and it has to be the one that initialized SDL.
/// The joystick subsystem and joystick handles cannot be moved across threads either.
/// This thread is thus responsible for pumping events,
/// forwarding all controller added/removed events to the comm manager,
/// and handling battery read and vibration write tasks.
fn sdl2_event_loop_thread(
  comm_sender: mpsc::Sender<HardwareCommunicationManagerEvent>,
  scanning_status: Arc<AtomicBool>,
) -> Result<(), String> {
  trace!("SDL2 event loop thread started");

  // Enable DS4 rumble.
  // SDL hint comments say that this turn on extended reports and thus mess up use of the DS4 for
  // apps that don't use SDL, until the DS4 is power-cycled.
  // TODO(Vyr): make this a config variable so that games that use the gamepad for input still work?
  //  How do we do that kind of prefs in Buttplug?
  sdl2::hint::set("SDL_JOYSTICK_HIDAPI_PS4_RUMBLE", "1");

  let sdl_context = sdl2::init()?;
  let joystick_subsystem = sdl_context.joystick()?;
  let mut event_pump = sdl_context.event_pump()?;

  // Map of joystick ID to hardware event sender for that joystick.
  let mut event_senders = HashMap::<u32, broadcast::Sender<HardwareEvent>>::new();

  let rt = tokio::runtime::Builder::new_current_thread()
    .thread_name("sdl2-event-loop-thread-rt")
    .enable_all()
    .build()
    .map_err(|e| {
      format!("SDL2 event loop thread couldn't create Tokio current thread runtime: {e}")
    })?;

  let local_set = LocalSet::new();

  rt.block_on(async {
    loop {
      let exit = local_set
        .run_until(sdl2_poll_event(
          &comm_sender,
          &mut event_pump,
          &joystick_subsystem,
          &mut event_senders,
          scanning_status.clone(),
        ))
        .await;
      if exit {
        break;
      }
    }
  });

  trace!("SDL2 event loop thread finished");
  Ok(())
}

/// Handle at most one possibly relevant SDL event.
/// Drives the event pump.
/// Returns true if we should stop processing SDL events.
async fn sdl2_poll_event(
  comm_sender: &mpsc::Sender<HardwareCommunicationManagerEvent>,
  event_pump: &mut EventPump,
  joystick_subsystem: &JoystickSubsystem,
  event_senders: &mut HashMap<u32, broadcast::Sender<HardwareEvent>>,
  scanning_status: Arc<AtomicBool>,
) -> bool {
  // Yield at least once so we have time to drive futures in the local set.
  task::yield_now().await;

  if let Some(event) = event_pump.poll_event() {
    match event {
      Event::JoyDeviceAdded { which: index, .. } => {
        debug!("SDL2 comm manager found a new joystick at index {index}");

        if comm_sender.is_closed() {
          trace!(
            "SDL2 comm manager event sender is closed. Skipping new joystick at index {index}"
          );
          return false;
        }

        if !scanning_status.load(Ordering::SeqCst) {
          trace!("SDL2 comm manager is not scanning. Skipping new joystick at index {index}");
          return false;
        }

        let joystick = match joystick_subsystem.open(index) {
          Ok(joystick) => joystick,
          Err(e) => {
            trace!("Couldn't open new joystick at index {index}: {e}");
            return false;
          }
        };

        if !joystick.has_rumble() {
          trace!("New joystick at index {index} does not support rumble, skipping it");
          return false;
        }

        let name = joystick.name();
        let id = joystick.instance_id();
        debug!("Opened new joystick at index {index} with ID {id}: {name}");

        let address = format!("{id}");
        let (event_sender, _) = broadcast::channel(256);

        event_senders.insert(joystick.instance_id(), event_sender.clone());

        let joystick_actor = SDL2JoystickActor::new(joystick);
        let joystick_actor_handle = joystick_actor.new_handle();

        task::spawn_local(
          async move {
            let mut joystick_actor = joystick_actor;
            joystick_actor.run().await
          }
          .boxed_local(),
        );

        if let Err(e) = comm_sender
          .send(HardwareCommunicationManagerEvent::DeviceFound {
            name: name.clone(),
            address: address.clone(),
            creator: Box::new(SDL2HardwareConnector::new(
              name,
              address,
              joystick_actor_handle,
              event_sender,
            )),
          })
          .await
        {
          error!("SDL2 event loop thread couldn't send connection event: {e}");
        }
      }

      Event::JoyDeviceRemoved { which: id, .. } => {
        debug!("SDL2 comm manager lost a joystick with ID {id}");
        if let Some(event_sender) = event_senders.remove(&id) {
          let address = format!("{id}");
          if let Err(e) = event_sender.send(HardwareEvent::Disconnected(address)) {
            error!("SDL2 event loop thread couldn't send disconnection event: {e}");
          }
        }
      }

      Event::Quit { .. } => {
        debug!("SDL is quitting");
        return true;
      }

      _ => {}
    }
  }

  return false;
}
