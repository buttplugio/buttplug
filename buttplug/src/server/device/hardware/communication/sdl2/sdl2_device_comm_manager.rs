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
      sdl2::sdl2_hardware::SDL2HardwareConnector,
      HardwareCommunicationManager,
      HardwareCommunicationManagerBuilder,
      HardwareCommunicationManagerEvent,
    },
    HardwareEvent,
  },
};
use async_trait::async_trait;
use futures_util::FutureExt;
use sdl2::{
  self,
  event::Event,
  joystick::{Joystick, PowerLevel},
  EventPump,
  IntegerOrSdlError,
  JoystickSubsystem,
};
use std::{
  collections::HashMap,
  fmt::{Debug, Formatter},
  future,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  thread,
};
use tokio::{
  sync::{broadcast, mpsc, oneshot},
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
        .name("sdl-event-loop-thread".to_owned())
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

trait SdlResultExt<T> {
  fn map_sdl_error(self) -> Result<T, String>;
}

impl<T> SdlResultExt<T> for Result<T, IntegerOrSdlError> {
  fn map_sdl_error(self) -> Result<T, String> {
    self.map_err(|e| format!("{e}"))
  }
}

/// Lives on the SDL2 event loop thread and responds to messages.
struct SDL2JoystickActor {
  joystick: Joystick,
  message_receiver: mpsc::Receiver<SDL2JoystickMessage>,
}

struct JoystickDebug<'a>(&'a Joystick);

impl Debug for JoystickDebug<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Joystick")
      .field("instance_id", &self.0.instance_id())
      .field("name", &self.0.name())
      .finish_non_exhaustive()
  }
}

impl Debug for SDL2JoystickActor {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("SDL2JoystickActor")
      .field("joystick", &JoystickDebug(&self.joystick))
      .field("message_receiver", &self.message_receiver)
      .finish()
  }
}

impl SDL2JoystickActor {
  fn handle_message(&mut self, message: SDL2JoystickMessage) {
    match message {
      SDL2JoystickMessage::Rumble {
        low_frequency_rumble,
        high_frequency_rumble,
        duration_ms,
        oneshot_sender,
      } => {
        // If the receiver's gone, we don't care if the send fails.
        let _ = oneshot_sender.send(
          self
            .joystick
            .set_rumble(low_frequency_rumble, high_frequency_rumble, duration_ms)
            .map_sdl_error(),
        );
      }
      SDL2JoystickMessage::PowerLevel { oneshot_sender } => {
        let _ = oneshot_sender.send(self.joystick.power_level().map_sdl_error());
      }
    }
  }

  async fn run(&mut self) {
    while let Some(msg) = self.message_receiver.recv().await {
      self.handle_message(msg);
    }
  }
}

/// Lives inside `SDL2Hardware` on any thread.
/// Sends and receives messages to its actor.
/// Sends disconnect events to the `SDL2Hardware`.
#[derive(Clone, Debug)]
pub struct SDL2JoystickActorHandle {
  message_sender: mpsc::Sender<SDL2JoystickMessage>,
}

impl SDL2JoystickActorHandle {
  async fn send_message_and_wait<T: Debug>(
    &self,
    message: SDL2JoystickMessage,
    oneshot_receiver: oneshot::Receiver<T>,
  ) -> Result<T, String> {
    self
      .message_sender
      .send(message)
      .await
      .map_err(|e| format!("SDL2 joystick actor proxy couldn't send message: {e}"))?;
    // TODO(Vyr): add a timeout here
    oneshot_receiver
      .await
      .map_err(|e| format!("SDL2 joystick actor proxy couldn't receive result: {e}"))
  }

  pub async fn rumble(
    &self,
    low_frequency_rumble: u16,
    high_frequency_rumble: u16,
    duration_ms: u32,
  ) -> Result<(), String> {
    let (oneshot_sender, oneshot_receiver) = oneshot::channel();
    self
      .send_message_and_wait(
        SDL2JoystickMessage::Rumble {
          low_frequency_rumble,
          high_frequency_rumble,
          duration_ms,
          oneshot_sender,
        },
        oneshot_receiver,
      )
      .await?
  }

  pub async fn power_level(&self) -> Result<PowerLevel, String> {
    let (oneshot_sender, oneshot_receiver) = oneshot::channel();
    self
      .send_message_and_wait(
        SDL2JoystickMessage::PowerLevel { oneshot_sender },
        oneshot_receiver,
      )
      .await?
  }
}

#[derive(Debug)]
enum SDL2JoystickMessage {
  Rumble {
    low_frequency_rumble: u16,
    high_frequency_rumble: u16,
    duration_ms: u32,
    oneshot_sender: oneshot::Sender<Result<(), String>>,
  },
  PowerLevel {
    oneshot_sender: oneshot::Sender<Result<PowerLevel, String>>,
  },
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
    .thread_name("sdl-event-loop-thread-rt")
    .enable_all()
    .build()
    .map_err(|e| {
      format!("SDL2 event loop thread couldn't create Tokio current thread runtime: {e}")
    })?;

  let local_set = LocalSet::new();

  rt.block_on(async {
    while !comm_sender.is_closed() {
      local_set
        .run_until(sdl2_poll_event(
          &comm_sender,
          &mut event_pump,
          &joystick_subsystem,
          &mut event_senders,
          scanning_status.clone(),
        ))
        .await;
    }
  });

  trace!("SDL2 event loop thread finished");
  Ok(())
}

/// Handle at most one possibly relevant SDL event.
/// Drives the event pump.
async fn sdl2_poll_event(
  comm_sender: &mpsc::Sender<HardwareCommunicationManagerEvent>,
  event_pump: &mut EventPump,
  joystick_subsystem: &JoystickSubsystem,
  event_senders: &mut HashMap<u32, broadcast::Sender<HardwareEvent>>,
  scanning_status: Arc<AtomicBool>,
) {
  // Yield at least once so we have time to drive futures in the local set.
  task::yield_now().await;

  if let Some(event) = event_pump.poll_event() {
    match event {
      Event::JoyDeviceAdded { which: index, .. } => {
        trace!("SDL2 comm manager found a new joystick at index {index}");
        if !scanning_status.load(Ordering::SeqCst) {
          trace!("SDL2 comm manager is not scanning, skipping new joystick at index {index}");
          return;
        }
        let joystick = match joystick_subsystem.open(index) {
          Ok(joystick) => joystick,
          Err(e) => {
            trace!("Couldn't open new joystick at index {index}: {e}");
            return;
          }
        };
        if !joystick.has_rumble() {
          trace!("New joystick at index {index} does not support rumble, skipping it");
          return;
        }
        let name = joystick.name();
        let id = joystick.instance_id();
        trace!("Opened new joystick at index {index} with ID {id}: {name}");

        let address = format!("{id}");
        let (message_sender, message_receiver) = mpsc::channel(256);
        let (event_sender, _) = broadcast::channel(256);

        event_senders.insert(joystick.instance_id(), event_sender.clone());

        task::spawn_local(
          async move {
            SDL2JoystickActor {
              joystick,
              message_receiver,
            }
            .run()
            .await
          }
          .boxed_local(),
        );

        let joystick_actor_handle = SDL2JoystickActorHandle { message_sender };
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
        // TODO(Vyr): this should exit the thread
        println!("SDL says byyyeeeeeeee!");
      }

      _ => {}
    }
  }
}
