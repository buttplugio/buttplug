// Buttplug SDL2 Gamepad Hardware Manager
//
// Cross-platform gamepad rumble/haptics support via SDL2.
// Works on macOS (GCController backend), Windows (XInput/DirectInput),
// and Linux (evdev) — all from a single codebase.
//
// Copyright 2026 chiefautism. BSD-3-Clause license.

#[macro_use]
extern crate log;

#[macro_use]
extern crate strum_macros;

mod sdl_gamepad_comm_manager;
mod sdl_gamepad_hardware;

pub use sdl_gamepad_comm_manager::{
  SdlGamepadCommunicationManager,
  SdlGamepadCommunicationManagerBuilder,
};
