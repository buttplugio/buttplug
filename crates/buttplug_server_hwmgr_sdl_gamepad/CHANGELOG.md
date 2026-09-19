# 12.0.1 (2026-09-19)

## Bugfixes

- Warm up SDL's GameController event loop on the main queue so macOS GUI hosts can receive MFI controller discovery events.

# 12.0.0 (2026-09-18)

## Breaking Changes

- Initial coordinated 12.x release aligned with the server and device-config trait/specifier changes. The removed standalone HID/XInput packages are not replaced by compatible package names; use this SDL gamepad manager instead.

## Features

- SDL3 gamepad rumble, battery reporting, capability-specific layouts, and the upstream SDL runtime refresh.
- Gamepads now report battery level through the standard buttplug battery command: a one-byte percent read on a new rx endpoint, sourced from SDL's gamepad power info (wired/no-battery and unknown states report an error instead of a value; charging states report their current percent).

## Bugfixes

- Rumble keepalives now actually reach the controller: SDL skips transmission of an unchanged (low, high) rumble pair, so keepalive re-arms alternate one motor channel's lowest bit (imperceptible) to force a real output report. The keepalive interval is also tightened from 1s to 100ms; Bluetooth DualSense and Joy-Con no longer stall effects mid-arm.
- Scanning announces each gamepad once per enumeration appearance instead of re-announcing unconnected devices on every scan tick, matching the btleplug manager's behavior.

# 11.0.0 (2026-09-05)

## Features

- Initial release. Cross-platform (Windows/macOS/Linux) gamepad rumble hardware manager for Buttplug, built on SDL3 via the `sdl3` crate (statically linked, built from source). One process-lifetime thread owns the SDL context and multiplexes all gamepads; devices are addressed by SDL3 instance ID (`sdl-gamepad-{instance_id}`) and present two 0-65535 vibrate features. Structural inspiration credit: chiefautism's abandoned PR #860.

## Platform notes

- macOS: **Bluetooth controllers only.** Wired pads are skipped at scan time with a logged explanation: Apple gives hidapi read-only shortened reports for wired gamepads, so rumble cannot work that way, and the working path (GCController) requires a main-thread runloop this architecture does not host. SDL2 shares this Apple limitation. Windows/Linux support wired and Bluetooth controllers.
- Rumble is armed finitely (60s) and re-armed every second as a keepalive (some controllers, e.g. Bluetooth DualSense, stop early despite a long arm); an explicit zero-speed stop is sent on close or removal.
