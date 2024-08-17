# 0.8.1 - 2024/08/17

## Bugfixes

- Fix issue with mismatch between enum field and unnamed type being different causing compile issues
  in try_from generation for server message structs.

# 0.8.0 - 2022/12/29

## Features

- Add ButtplugMessageFinalizer trait derivation

# 0.7.0 - 2022/08/29

## Changes

- Remove ButtplugProtocolProperties - No longer used in main library

# v0.6.2 - 2021/02/04

## Changes

- Update buttplug message and device message with simplified getter method names.

# v0.6.1 - 2021/01/21

## Changes

- Update dependencies since this library almost never changes anyways.

# v0.6.0 - 2021/01/21

## Changes

- Renamed MessageAttributes to DeviceMessageAttributes, which required an update in the protocol
  properties derive macro.

# v0.5.0 - 2020/10/09

## Features

- Remove ButtplugProtocolCreator and ButtplugProtocol derives
  - ButtplugProtocolCreator is no longer a thing, and ButtplugProtocol can
    no longer be derived.

# v0.4.0 - 2020/07/26

## Features

- Fix derives to use new thiserror errors (in buttplug 0.5).

# v0.3.0 - 2020/06/22

## Features

- Add derives for:
  - ButtplugServerMessage
  - ButtplugClientMessage
  - ButtplugProtocol
  - ButtplugProtocolProperties
  - ButtplugProtocolCreator

# v0.2.0 - 2020/04/12

## Features

- Added derives for union generation and message conversion.

# v0.1.0 - 2020/02/15

## Features

- Update dependencies
- Add proc macros for enum trait derivation

# v0.0.1 - 2019/11/03

## Features

- Implementation of ButtplugMessage trait derivation macro
