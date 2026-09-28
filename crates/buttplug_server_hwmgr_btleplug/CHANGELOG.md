# 12.0.2 (2026-09-27)

## Bugfixes

- Update btleplug to 0.13.3
  - Fix Android JNI adapter race causing SIGSEGV/SIGBUS crashes
  - Fix Android connect failures (GATT 133), hung command queues on disconnect, and GATT read/write failures reported as success
  - Fix CoreBluetooth hangs and panics during service discovery, concurrent connect/disconnect, and service invalidation
  - Fix Windows deadlock on concurrent GATT operations, GATT failures after reconnect, and scanner crash on truncated service data
  - Fix BlueZ `mtu()` panic on BlueZ older than 5.62 and unsubscribe errors on unsubscribed characteristics

# 12.0.1 (2026-09-20)

## Bugfixes

- Update btleplug to fix leak in windows when de/allocating adapters constantly

# 12.0.0 (2026-09-18)

## Breaking Changes

- Rebuild public manager and connector integrations against the coordinated 12.x server and device-config contracts.

# 11.0.0 (2026-07-28)

## Other

- Update buttplug crates to 11.0.0

# 10.0.4 (2026-06-01)

## Features

- Update internal Buttplug library dependencies

# 10.0.3 (2026-05-31)

## Bugfixes

- Guard disconnect handling against hangs on already-disconnected devices

# 10.0.2 (2026-04-01)

## Features

- Migrate to new async_manager API

# 10.0.1 (2026-03-13)

## Features

- Update btleplug to v0.12
  - Lots of Android fixes, should throw more exceptions instead of silent failures
  - Lots of other bugfixes

# 10.0.0 (2026-01-31)

## Features

- Update dependencies

# 10.0.0-beta3 (2025-12-26)

## Features

- Update dependencies

# 10.0.0-beta1 (2025-10-12)

## Features

- Split hardware manager library into own crate
- That's it really, hardware managers didn't change much this revision

# Earlier Versions

- See [Buttplug Crate CHANGELOG.md](../buttplug/CHANGELOG.md)
