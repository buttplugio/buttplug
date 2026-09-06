# 11.0.0 (2026-09-05)

## Features

- Initial release. Cross-platform (Windows/macOS/Linux) gamepad rumble hardware manager for Buttplug, built on SDL3 via the `sdl3` crate (statically linked, built from source). One process-lifetime thread owns the SDL context and multiplexes all gamepads; devices are addressed by SDL3 instance ID (`sdl-gamepad-{instance_id}`) and present two 0-65535 vibrate features. Structural inspiration credit: chiefautism's abandoned PR #860.
