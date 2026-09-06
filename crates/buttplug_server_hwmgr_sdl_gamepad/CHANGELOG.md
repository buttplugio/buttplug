# 11.0.0 (2026-09-05)

## Features

- Initial release. Cross-platform (Windows/macOS/Linux) gamepad rumble hardware manager for Buttplug, built on SDL3 via the `sdl3` crate (statically linked, built from source). One process-lifetime thread owns the SDL context and multiplexes all gamepads; devices are addressed by SDL3 instance ID (`sdl-gamepad-{instance_id}`) and present two 0-65535 vibrate features. Structural inspiration credit: chiefautism's abandoned PR #860.

## Platform notes

- macOS: **Bluetooth controllers only.** Wired pads are skipped at scan time with a logged explanation: Apple gives hidapi read-only shortened reports for wired gamepads, so rumble cannot work that way, and the working path (GCController) requires a main-thread runloop this architecture does not host. SDL2 shares this Apple limitation. Windows/Linux support wired and Bluetooth controllers.
- Rumble is armed finitely (60s) and refreshed before expiry; an explicit zero-speed stop is sent on close or removal.
