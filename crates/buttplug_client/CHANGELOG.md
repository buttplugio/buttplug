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