// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Cross-platform (Windows/macOS/Linux) gamepad rumble hardware manager for
//! Buttplug, built on SDL3.
//!
//! A single process-lifetime thread owns the SDL3 context and multiplexes all
//! gamepads; discovery is on-demand SDL gamepad enumeration and removal
//! detection is per-device connected-state polling. No SDL events are pumped
//! (SDL3 documents `SDL_PumpEvents` as main-thread-only, and this manager
//! does not consume controller input).
//!
//! Use `--use-sdl-gamepad` with intiface-engine; in buttplug_client_in_process,
//! the `sdl-gamepad-manager` cargo feature is part of the default feature set.

#[macro_use]
extern crate log;

#[cfg(target_os = "macos")]
mod game_controller_warmup;
mod sdl_comm_manager;
mod sdl_gamepad_hardware;
mod sdl_task;

pub use sdl_comm_manager::{SdlGamepadCommunicationManager, SdlGamepadCommunicationManagerBuilder};
