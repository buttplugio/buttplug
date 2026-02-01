# 10.0.0 (2026-01-31)

## Features

- Replace ButtplugFuture w/ Oneshot channels
  - We'd basically made a half-assed version of that anyways
- Remove buttplug_derive completely
- Changes around v4 spec finalization
  - PositionWithDuration -> HwPositionWithDuration
  - StopDeviceCmd/StopAllDevices -> StopCmd

# 10.0.0-beta4 (2025-12-29)

## Features

- Implement Inputs
- Update name of Input property fields for DeviceList
  - This will be a breaking change between beta3 and beta4

## Bugfixes

- Actually check outgoing messages against JSON schema

# 10.0.0-beta3 (2025-12-26)

## Features

- Simplify command structures to using embedded enums
- Remove DeviceAdded/DeviceRemoved events, now just send DeviceListUpdated and let clients parse as
  necessary.
- Update dependencies

# 10.0.0-beta2 (2025-10-18)

## Features

- Change heater type to temperature

# 10.0.0-beta1 (2025-10-12)

## Features

- Client moved to its own crate
- API updated to work with v4 message spec
- API allows both discrete steps and float based commands

# Earlier Versions

- See [Buttplug Crate CHANGELOG.md](../buttplug/CHANGELOG.md)