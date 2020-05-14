# 0.3.1 (2020-05-13)

## Bugfixes

- Error instead of panic on btleplug connection failure
- Explain issues with scanning failures in error message

# 0.3.0 (2020-05-10)

## Features

- Servers can now be extracted from wrappers, meaning they can be
  saved and reused between connections. This allows us to implement
  the --stayopen argument in intiface-cli.
  
## Bugfixes

- Make tests broken in 0.2.4 actually pass again.

# 0.2.4 (2020-05-09)

## Bugfixes

- Fixed issue with ServerInfo not returning proper matching message ID
  when communicating with clients of older spec versions
- Fixed issue with in-process server wrappers not setting message ID
  on return

# 0.2.3 (2020-04-18)

## Features

- ButtplugServerJSONWrapper can now use injected servers instead of always
  creating the server itself

## Bugfixes

- Fix global device config string types
- Update to btleplug 0.4.2, which no longer prints to console on windows
- XInput no longer prints everything to console

# 0.2.2 (2020-04-15)

## Features

- XInput is now a default feature
  - This needed to happen in order to make things like intiface-cli
    easier to build. Doesn't affect any non-windows platforms, as
    everything if cfg'd out.

# 0.2.1 (2020-04-14)

## Bugfixes

- Update the README. Really. That's it. I just forgot. :(

# 0.2.0 (2020-04-12)

## Features

- Added XInput support on windows
- Added TestDeviceCommunicationManager and devices for testing/examples.
- Add RequestLog/Log handling in server.
- Create Server Wrappers concept, for handling message conversion into/out of
  the server. Includes JSON implementation.
- Make Server backward compatible to connections from all older message spec
  versions (using server wrappers), along with ability to do message
  up/downgrades (i.e. VibrateCmd <-> SingleMotorVibrateCmd for spec 0/1)
- Add JSON schema verification of device config files and incoming messages on
  client/server
- Add simple user configuration loading, so users can specify serial ports once
  we support them

## Bugfixes

- Ping timeouts now actually stop devices

## Other

- Divided out message classes into their own files
- Broke ButtplugMessageUnion into smaller message unions, so we can do less type
  checking manually.

# 0.1.0 (2020-02-15)

## Features

- Added server, with Win/Linux/macOS/iOS access to Bluetooth
- Added device support for the following brands:
  - Lovense
  - Picobong
  - Aneros
  - Lovehoney
  - MaxPro
  - PrettyLove
  - Realov
  - Svakom
  - Vorze
  - YouCups
  - Youou
- Server not yet feature complete, missing functionality present in
  C#/Typescript servers. This is an intermediate release to allow
  testing of the server code while work on features and parity with
  other versions continues.

# v0.0.2 - 2019/11/21

## Features

- Overhauled API, now have an event loop that the client is created in
  and lives in.
- Finished full implementation of Client.
- Added features for partial library builds.
- Integrated websocket connector into library.
- More documentation.

# v0.0.2-beta.1 - 2019/11/03

## Features

- Lots of documentation.
- Continue cleaning up experimental implementation.
- Readd server tests.

## Bugfixes

- Connector failure now fails all the way up through the client
  instance.

# v0.0.2-beta.0 - 2019/11/02

## Features

- First version of the new rust rewrite. Uses async/await and Rust
  1.39 heavily.
- Partial client implementation.

# v0.0.1 - 2019/04/18

## Features

- Squatting the name on crates.io because I am horrible.

# v0.0.0 - 2016/10/01

- The original try at building Buttplug in Rust. At the time, tokio
  had just come out, and futures were just becoming a thing. A
  combination of those problems combined with lack of Bluetooth access
  support (which wouldn't even be possible on Windows 10 until April
  2017) meant this version fizzled out pretty quick.
