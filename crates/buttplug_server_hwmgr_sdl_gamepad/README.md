# buttplug_server_hwmgr_sdl_gamepad

Cross-platform (Windows/macOS/Linux) gamepad rumble hardware manager for
[Buttplug](https://buttplug.io), built on SDL3 via the `sdl3` Rust crate.

Gamepads appear as Buttplug devices with two 0-65535 vibrate features (low and
high frequency rumble motors), identified by the `sdl-gamepad` protocol. Each
device is addressed by its SDL3 instance ID (`sdl-gamepad-{instance_id}`),
which is stable for the lifetime of the connection.

This manager is **opt-in everywhere**:

- In `intiface-engine`, pass `--use-sdl-gamepad`.
- In `buttplug_client_in_process`, enable the non-default
  `sdl-gamepad-manager` cargo feature.

## How it works

A single process-lifetime thread owns the SDL3 context. All gamepads are
multiplexed through it: discovery is on-demand `SDL_GetGamepads` enumeration
and removal detection is per-device connected-state polling. The thread never
pumps SDL events (SDL3 documents `SDL_PumpEvents` as main-thread-only, and
this manager does not consume controller input).

Rumble is armed with a finite duration and refreshed by the SDL thread before
expiry, so one-shot ScalarCmd commands hold indefinitely. The sdl3 crate
documents that `u32::MAX` durations overflow and end the effect immediately,
so infinite durations are never used.

## Build prerequisites

The `sdl3` dependency uses the `build-from-source-static` feature: SDL3 is
downloaded and built (and statically linked) at crate build time. This
requires `cmake` and a C compiler on the build machine:

- macOS: Xcode command line tools (`xcode-select --install`)
- Linux: `gcc`/`clang` and `cmake` (plus the usual development headers for a
  headless SDL3 build; on Debian/Ubuntu `build-essential` and `cmake` suffice
  for the joystick/gamepad subsystem)
- Windows: Visual Studio C++ build tools and `cmake` (both present on GitHub
  Actions windows runners)

Static linking keeps the single-binary release pipeline unchanged; expect the
resulting binary to grow by a few MB.

## Testing without hardware

CI runners have no physical gamepads and the `sdl3` crate has no simulation
layer. All buttplug-side behavior (discovery, addressing, command forwarding,
lifecycle) is unit-tested in this crate against mock drivers/backends. The
SDL-thread interior is exercised by the `examples/sdl3_thread_spike.rs`
example (headless init + no-pump enumeration on a spawned thread), which CI
runs on all three operating systems. Real-controller behavior — notably
smooth continuous rumble with the refresh-before-expiry scheme — must be
validated manually on each platform before release; see the manual validation
checklist in the repository's pull request for this feature.

## Coexistence with XInput

On Windows, both the XInput manager and this manager can be enabled at the
same time; the same physical controller may then appear as two Buttplug
devices (once via each manager). `intiface-engine` logs a warning when both
flags are set. Outside Windows, only this manager is available.
