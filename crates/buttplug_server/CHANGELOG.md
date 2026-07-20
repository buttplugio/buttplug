## Unreleased

## Compatibility Notice

- `ProtocolHandler::handle_input_subscribe_cmd` now receives a `TaskScope` argument. This is technically a public Rust API signature change, so custom external implementations must update their method signature.
- `ProtocolHandler` is currently intended for protocol implementations shipped inside this repository; it is not treated as a stable external extension interface. In line with ADR 0008, servers are not intended to be independently reimplemented. This policy does not make the public trait change non-breaking for existing external implementations.

# 10.0.4 (2026-06-01)

## Features

- Update to device configuration v10.1.1 with additional device support

## Bugfixes

- Add new JoyHub Rosella 2 and Svakom Pulse Lite Neo identifiers
- Add heater support to JoyHub Thermos

# 10.0.3 (2026-05-31)

## Features

- Add output observation streams for emitted device commands
- Add simulated hardware manager, protocol handler, and runtime simulated device configuration
- Add Utimi protocol support
- Device protocol support
  - JoyHub Valora (J-Volt)
  - JoyHub Rowdy
  - Utimi KnotVibe ThrustMaster
  - Additional Lelo, HoneyPlayBox, and Lovense devices

## Bugfixes

- Fix Lovense stroker stop handling and battery timeout behaviour
- Fix legacy output vector feature bounds checks
- Fix several device protocol command paths and compiler warnings

# 10.0.2 (2026-04-01)

## Features

- Migrate to new async_manager API
- Device protocol support
  - Various JoyHub devices
  - TryFun Rock
  - MyMuse Link Plus

## Bugfixes

- Fix V0/V1 protocol client errors for testing
- Fix Scanning state machine getting stuck, stopping rediscovery of disconnected devices
- Fix message ID generation for DeviceListV4 message

# 10.0.1 (2026-03-13)

## Features

- Update dependencies
- Device updates
  - Add battery reading for WeVibe
- Device protocol support
  - HoneyPlayBox devices
  - Various JoyHub devices
- Expose "needs_keepalive" to engine to minimize mobile wakelocks

# 10.0.0 (2026-01-31)

## Features

- Lots of Cleanup
  - Remove buttplug_derive proc macros
  - Rebuild device system to use 3 fewer long-running tasks
  - Start reworking server systems into a state machine architecture
    - This will continue post v10, we just need to ship right now
- Device Updates
  - Add temperature/LED support for joyhub devices
  - Fix svakom devices
- Fixes for Buttplug v4 Spec finalization
  - StopAllDevices/StopDeviceCmd -> StopCmd
  - PositionWithDuration -> HwPositionWithDuration

# 10.0.0-beta4 (2025-12-29)

## Features

- Update name of Input property fields for DeviceList
  - This will be a breaking change between beta3 and beta4

## Bugfixes

- Actually check outgoing messages against JSON schema

# 10.0.0-beta3 (2025-12-26)

## Features

- Implement explicit feature indexes for device configs
  - This will be a breaking change between beta2 and beta3
- Added device support
  - The Handy 2 Pro
  - The Oh!
  - Adorime Pink Touch
  - WeVibe Sync O
  - JoyHub Torque
  - JoyHub Mighty
  - JoyHub Violet Gale
  - Vorze Omorfi  
  - Sensee Markel
  - Easylive Gamer
  - New(?) Nobra Controllers
  - Fluffer devices (new MotorBunnys)
  - Adorime Cock Ring
- Update dependencies

## Bugfixes

- Simplify joyhub impls using feature indexes
- Fix Oscillation Range impl for Lovense Solace Pro
- Actually implement InputCmd :|

# 10.0.0-beta2 (2025-10-18)

## Features

- Change heater type to temperature

# 10.0.0-beta1 (2025-10-12)

## Features

- Added Device Support:
  - Fredorch F2S1 (Updated controls algorithm)
  - Kiiroo PleasureDrive, Powershot
  - Hismith/Sinloli Piupiu (lube injector)
  - Sinloli Aston
  - Joyhub Persues, Divers, Peachy
  - Various Sexverse devices
  - Lovemazer devices
  - Duopeak Saphette
  - Adorime Penis Helmet Vibrator, Chastity Cage, Backy
  - Qingnan devices
  - Hannibal Kona
  - Pink Punch Peacaron
- Server split into its own crate
- Updated messages for v4
- Deprecated Messages from v0-v3 now only exist in the server, since they're only needed for
  backward compat.
- Completely rewrote the message conversion system for backward compat

# Earlier Versions

- See [Buttplug Crate CHANGELOG.md](../buttplug/CHANGELOG.md)
