# 0.8.0 (2020-09-13)

## Features

- Simplified logging, adding a channel output filter while removing
  request_log/log client access for the moment.
  - Minor version update due to public facing API surface change.
  - Not particularly sure about the future of logging in the Buttplug Protocol,
    as it's both a security risk and difficult to work with. It seems like it
    might be better to expunge logging from the protocol and just deal with it
    at the library/app level. See
    https://github.com/buttplugio/buttplug-rs/issues/131 for more discussion.

## Bugfixes

- Lovense rotation for Nora works again
- Websocket Server should stop panicing on unwrap when connection is not severed
  gracefully (#153)

# 0.7.3 (2020-09-11)

## Bugfixes

- Allow for loading either PKCS8 or RSA private keys in websocket secure server
  impl

# 0.7.2 (2020-09-07)

## Features

- Merge device configuration changes from mainline device config
  - Should fix up some issues with Lovense devices not appearing.

## Bugfixes

- Websockets now actually listen on all interfaces when option is passed.

# 0.7.1 (2020-09-06)

## Bugfixes

- Fix documentation location so docs.rs doesn't get mad about missing docs.

# 0.7.0 (2020-09-06)

## Features

- Added device support:
  - Kiiroo v2 (Onyx 2/Launch), v2.1 (Onyx+/Cliona/Titan/Blowbot/Pearl 2/Pearl 2.1 etc)
  - Vibratissimo (all products)
  - Motorbunny
  - WeVibe
  - Magic Motion
- Expose device index on client devices for FFI usage
- Have remote server emit events for Intiface CLI

## Bugfixes

- Fix LinearCmd JSON schema
- Fix mismatched stepcounts for some devices in device config
- Make device command rounding use ceiling, mirroring output of
  buttplug-js/buttplug-csharp

# 0.6.0 (2020-08-03)

## Features

- Add websocket server transport, allowing a buttplug client/server to sit
  behind either a websocket client or server.
- Add Remote Server class, allowing a buttplug server to be wrapped in a
  serializer and transport for remote use (i.e. over websockets, tcp, etc), as
  well as being reused between connections.

## Bugfixes

- Fix names of Lovense Dongle managers so they don't write over each other.
- Fix checking of comm managers being added so two managers of the same
  name/type can't be added.
- Lovense HID Dongle manager no longer panics if it can't find a dongle.
- Import README as top level doc
- Fix library building with feature variations (i.e. no features, client only,
  server only, etc)

## Maintenance

- Change feature names to all be kebab-case, and to follow certain standards
  (i.e. all comm manager features end in "-manager")

# 0.5.0 (2020-07-26)

## Features

- Error system now uses thiserror
  - Should be more consistent and provide more useful errors. I hope. Maybe.
- Serial port support added
  - No protocols using this yet, TCode/ET312/etc coming in point releases.
- Lovense dongle support added
  - Handles both serial and HID versions of the dongle, on all desktop
    platforms.

## Bugfixes

- Bluetooth Device Disconnections now notify on all native platforms
  - Used to be just linux, now fixed for mac/win also
- Stopping scanning twice no longer panics
- Fix start scanning timing to happen when future is await'd
  - This could cause discovery race conditions in the past

# 0.4.0 (2020-06-21)

## Features

- Logging now handled via tracing.
  - More work needed to get all futures instrumented and what not, but we're on
    the way.
- Connector module created, ungluing connectors from the Client API and making
  them Client/Server agnostic. Wrappers from 0.3.0 merged into the connector
  implementation.
  - Server connectors on the way in an upcoming version.
- Abstract serializers into connector module
  - Serializers were split cross Client and Server implementations. Like
    connectors, they are now agnostic to Client/Server usage.
- Abstract runtime management to async_manager
  - Idea taken from https://github.com/najamelan/async_executors, but had some
    different requirements. May switch to that at some point though.
- Add more documentation for Client and Connector modules
- Simplify event loops some
  - The internal event loops were becoming a rats nest of channels, select!'s
    and match blocks. They're still not great, but they're better than they
    were.
- Move to dashmap for internal concurrent Hashmaps
  - Tried evmap, but don't need multi-valued maps. Dashmap is a good
    intermediary between evmap and Arc<Mutex<Hashmap<T>>>
- Implement RawRead/Write/Subscribe/Unsubscribe
  - More Buttplug v2 message spec messages. Still not currently exposed on
    devices as we need it to be an opt-in feature.

## Bugfixes

- Clarify names of messages structs
  - ButtplugIn/OutMessage, while hilariously on-brand for the project, didn't
    provide enough context in code. Renamed to ButtplugClient/ServerMessage,
    which denotes the originating source of the message (since Clients and
    Servers will never send the same message types).
- Async functions now actually async.
  - In earlier versions, most async methods took &mut self, meaning we were not
    async since usage of a struct would be locked while the future executed. As
    of 0.4.0, most if not all exposed methods take &self and return a future,
    meaning the library mostly works through lazy execution now.
- Protocols are no longer opaque macro structures
  - Went a little overboard on macros with protocols, meaning it was extremely
    difficult to tell what code was being generated, and it made debugging a
    nightmare. Protocols are now just structs with certain derivable traits with
    default impls, meaning functionality can be implemented in overrides which
    keeps the code clean and mostly free of boilerplate.
- Fixed race condition bug on device creation
  - We fired connection events before storing off the device, meaning depending
    on task scheduling clients could access devices they just got in DeviceAdded
    messages, that would then report as not found.

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
