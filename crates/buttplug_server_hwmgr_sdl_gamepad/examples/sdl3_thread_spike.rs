// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

// Phase 0 threading spike (automated half).
//
// Verifies the machinery the SDL gamepad manager relies on:
// - sdl3::init() + gamepad subsystem initialize on a dedicated spawned thread
//   (not the process main thread), headless (no video subsystem).
// - gamepads() enumerates on demand without any SDL event pumping (an empty
//   set is acceptable; CI runners have no controllers).
// - the thread's poll tick runs without crashing.
//
// This cannot prove real-controller behavior; that is the manual, per-OS half
// of the spike documented in the crate README.
use std::thread;
use std::time::Duration;

fn main() {
  // Built-in verbose SDL logging so a single run of this example is a
  // complete diagnostic (no SDL_LOGGING env var needed) - essential for
  // diagnosing backend claiming on other platforms.
  sdl3::log::set_log_priorities(sdl3::log::Priority::Verbose);
  let handle = thread::Builder::new()
    .name("sdl3-spike".to_string())
    .spawn(|| {
      println!("[sdl-thread] setting JOYSTICK_ALLOW_BACKGROUND_EVENTS hint (pre-init)");
      sdl3::hint::set(sdl3::hint::names::JOYSTICK_ALLOW_BACKGROUND_EVENTS, "1");
      // Mirror the production factory's platform policy (see
      // production_sdl_factory in src/sdl_task.rs for the full rationale).
      #[cfg(target_os = "macos")]
      sdl3::hint::set(sdl3::hint::names::JOYSTICK_MFI, "0");
      println!("[sdl-thread] sdl3::init()");
      let sdl = sdl3::init().expect("sdl3::init() must work on a dedicated thread");
      println!("[sdl-thread] init OK; initializing gamepad subsystem (headless)");
      let gamepad = sdl
        .gamepad()
        .expect("gamepad subsystem must initialize headless");
      println!("[sdl-thread] gamepad subsystem OK");
      match gamepad.gamepads() {
        Ok(ids) => {
          println!("[sdl-thread] gamepads() -> {} gamepad(s)", ids.len());
          // Connection state is what the macOS wired-skip keys on; printing
          // it makes every spike run a complete transport diagnostic.
          for id in &ids {
            match gamepad.open(*id) {
              Ok(pad) => {
                let connection = match pad.connection_state() {
                  Ok(sdl3::joystick::ConnectionState::Wired) => "Wired",
                  Ok(sdl3::joystick::ConnectionState::Wireless) => "Wireless",
                  Ok(_) => "Unknown",
                  Err(e) => {
                    println!(
                      "[sdl-thread] gamepad {} connection query failed: {e:?}",
                      id.0
                    );
                    "Error"
                  }
                };
                println!(
                  "[sdl-thread] gamepad {} '{}' connection: {}",
                  id.0,
                  pad.name().unwrap_or_default(),
                  connection
                );
                // pad drops here, closing the probe handle
              }
              Err(e) => println!("[sdl-thread] gamepad {} open failed: {e:?}", id.0),
            }
          }
        }
        Err(e) => {
          eprintln!("[sdl-thread] gamepads() failed: {e:?}");
          std::process::exit(2);
        }
      }
      for i in 0..20 {
        let ids = gamepad.gamepads().expect("gamepads() during tick");
        if i % 5 == 0 {
          println!("[sdl-thread] tick {}: {} gamepad(s)", i, ids.len());
        }
        thread::sleep(Duration::from_millis(100));
      }
      println!("[sdl-thread] spike passed");
    })
    .expect("spawn sdl thread");
  handle.join().expect("sdl thread join");
  println!("PASS");
}
