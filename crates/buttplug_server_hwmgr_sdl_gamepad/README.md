# buttplug_server_hwmgr_sdl_gamepad

Cross-platform (Windows/macOS/Linux) gamepad rumble hardware manager for
[Buttplug](https://buttplug.io), built on SDL3 via the `sdl3` Rust crate.

Gamepads appear as Buttplug devices using the `sdl-gamepad` protocol. Each
connection exposes one of these logical channel layouts, selected from SDL's
reported capabilities (which can vary by operating system and transport), not
from a device name or model:

- **Main-only:** Low-frequency rumble and High-frequency rumble (the classic
'two-channel' layout).
- **Trigger-only:** Left-trigger rumble and Right-trigger rumble at visible
  indexes 0 and 1.
- **Both:** all four channels in the order above.

Each channel is a vibrate feature with a range of 0-65535. A device reporting
neither main rumble nor trigger rumble is skipped. Capability probes are pure
property queries; the device is not permanently excluded, and a later scan
retries it.

Names come from SDL's `name_for_id` lookup. If lookup fails or returns only
whitespace, the deterministic fallback is `SDL Gamepad {instance_id}`. The
Buttplug address is based on the SDL instance ID. Identity is connection-scoped:
an instance ID and reported name can change after reconnecting, so settings
follow the connection-scoped identity rather than guaranteed physical-hardware
identity. Layout selection is also per connection; changed capabilities take
effect on the next connection.

When a layout changes, surviving channels retain their user UUIDs, limits,
disabled state, and display-name overrides. Channels removed by the layout
change lose their customizations permanently; if those channels reappear on a
later connection, they start with defaults. Feature descriptions come from the
device configuration on load and are not serialized in saved user configs.

## How it works

A single process-lifetime thread owns the SDL3 context. All gamepads are
multiplexed through it: discovery is on-demand `SDL_GetGamepads` enumeration
and removal detection is per-device connected-state polling. The thread never
pumps SDL events (SDL3 documents `SDL_PumpEvents` as main-thread-only, and this
manager does not consume controller input).

Internally, the protocol-to-hardware packet is 8 bytes: four little-endian
`u16` logical slots in fixed order `[low, high, left_trigger, right_trigger]`.
This is an internal transport detail, not a public wire-protocol change. Only
capability-supported pairs are dispatched to SDL, including zero, stop, and
keepalive commands.

Rumble is armed with a finite duration because the `sdl3` crate documents that
`u32::MAX` durations overflow and end the effect immediately. The ownership
thread refreshes each active main or trigger pair independently every second
before expiry; zero pairs stop refreshing. This keepalive makes one-shot
commands remain active on controllers that otherwise stop rumbling after a few
seconds.

Trigger output here is simple SDL trigger rumble via
`SDL_RumbleGamepadTriggers` (currently Xbox-One-class support). It is not
adaptive-trigger resistance or a resistance/force-feedback control.

## Build prerequisites

The `sdl3` dependency uses the `build-from-source-static` feature: SDL3 is
downloaded and built (and statically linked) at crate build time. This requires
`cmake` and a C compiler on the build machine:

- macOS: Xcode command line tools (`xcode-select --install`)
- Linux: `gcc`/`clang` and `cmake` (plus the usual development headers for a
  headless SDL3 build; on Debian/Ubuntu `build-essential` and `cmake` suffice
  for the joystick/gamepad subsystem)
- Windows: Visual Studio C++ build tools and `cmake`

Static linking keeps the single-binary release pipeline unchanged; expect the
resulting binary to grow by a few MB.

## Testing without hardware

CI runners have no physical gamepads and the `sdl3` crate has no simulation
layer. Buttplug-side behavior (discovery, addressing, command forwarding,
lifecycle, and layout selection) is unit-tested in this crate against mock
drivers/backends. The `examples/sdl3_thread_spike.rs` diagnostic prints the
SDL-reported name, connection state, and both capability booleans without
actuating anything.

## Manual release validation

On a supported platform and transport, validate both a **main-only** pad and a
**trigger-capable** pad:

1. Confirm the SDL-derived name (or deterministic fallback) and independent
   channels are visible in a current client.
2. Sustain main rumble long enough to cross multiple one-second keepalive
   refreshes; confirm it remains active.
3. On the trigger-capable pad, sustain trigger rumble across keepalive refreshes
   and confirm left and right trigger channels independently.
4. Send stop commands and confirm both rumble pairs stop.
5. Disconnect the pad and confirm the client receives disconnection.

Physical checks had **not** been performed as of this change. Automated tests
cannot prove motor behaviour or the exact number of physical actuators.

Confirmed on hardware so far: Bluetooth DualSense on macOS discovers and rumbles
(with the one-second keepalive re-arming the effect). This pre-existing note
must not be used to infer trigger-rumble support.

## Platform support

- **Windows / Linux**: wired and Bluetooth controllers via SDL's hidapi and
  platform backends.
- **macOS**: **Bluetooth controllers only.** Apple exposes wired gamepads to
  hidapi with read-only shortened HID reports, so rumble is impossible that
  way; working wired rumble requires GCController, whose discovery only fires
  from a main-thread runloop that this library deliberately does not host.
  Wired pads are skipped at scan time with a logged explanation - pair the same
  controller via Bluetooth for full support. (A future main-thread integration
  could lift this; the limitation is Apple's, and SDL2 shares it.)

## Coexistence with XInput

On Windows, both the XInput manager and this manager can be enabled at the same
time; the same physical controller may then appear as two Buttplug devices
(once via each manager). `intiface-engine` logs a warning when both flags are
set. Outside Windows, only this manager is available.

## Registration

This manager is **opt-in everywhere**:

- In `intiface-engine`, pass `--use-sdl-gamepad`.
- In `buttplug_client_in_process`, enable the non-default
  `sdl-gamepad-manager` cargo feature.
