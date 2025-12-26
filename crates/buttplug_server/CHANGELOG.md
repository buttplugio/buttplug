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