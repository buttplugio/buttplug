# 12.0.0 (2026-09-18)

## Breaking Changes

- Remove the public `xinput-manager` feature and standalone XInput manager dependency; migrate consumers to `sdl-gamepad-manager`.
- Add SDL gamepad to the default feature set, so default builds register SDL3 gamepads instead of XInput hardware.
- The in-process client now uses the coordinated 12.x server, device-config, and manager contracts.


# 11.0.0 (2026-07-28)

## Other

- Update buttplug crates to 11.0.0

# 10.0.4 (2026-06-01)

## Features

- Update internal Buttplug library dependencies

# 10.0.3 (2026-05-31)

## Features

- Update internal Buttplug library dependencies

# 10.0.2 (2026-04-01)

## Features

- Update to new async_manager spawn macros

# 10.0.1 (2026-03-13)

## Features

- Update dependencies

# 10.0.0 (2026-01-31)

## Features

- Update dependencies

# 10.0.0-beta3 (2025-12-26)

## Features

- Update dependencies

# 10.0.0-beta1 (2025-10-12)

## Features

- In-process client moved to its own crate, mostly because of the dependency complexity

# Earlier Versions

- See [Buttplug Crate CHANGELOG.md](../buttplug/CHANGELOG.md)
