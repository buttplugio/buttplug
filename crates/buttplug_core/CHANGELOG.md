# 10.0.0 (2026-01-31)

## Features

- Lots of cleanup
  - Remove buttplug_derive, add more boilerplate derives but no more proc macros
  - Other general macro cleanup
  - Various message renames for the final v4 spec
    - StopAllDevices/StopDeviceCmd -> StopCmd
    - PositionWithDuration -> HwPositionWithDuration

# 10.0.0-beta4 (2025-12-29)

## Features

- Update name of Input property fields for DeviceList

## Bugfixes

- Actually check outgoing messages against JSON schema

# 10.0.0-beta3 (2025-12-26)

## Features

- Update dependencies
- Update names of Output property fields for DeviceList

# 10.0.0-beta2 (2025-10-18)

## Features

- Change heater type to temperature

# 10.0.0-beta1 (2025-10-12)

## Features

- Core split into its own crate
- Updated messages for v4
- Core now only contains messages valid for the current version of the Buttplug Spec
- JSON Message schema now built on program build, since if that doesn't build, neither will the rest
  of the library

  # Earlier Versions

- See [Buttplug Crate CHANGELOG.md](../buttplug/CHANGELOG.md)

