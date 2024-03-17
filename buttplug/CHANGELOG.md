# 7.1.15 (2024-03-17)

## Features

- Added Device Support
  - Joyhub Firedragon

## Bugfixes

- Turn several .expect()'s into warn!() log messages
  - Mostly in Lovense Dongle handling. A lot of people have this on without knowing it and it
    doesn't react well to being shut down right now. We get several thousand sentry reports a week about this. God I fucking hate the Lovense Dongle.

# 7.1.14 (2024-03-16)

## Features

- Added Device Support
  - Sensee Capsule
  - MysteryVibe Legato
  - Meese Modo
  - JoyHub Elixir Egg, Retro Guard, Vib Trefoil, Pearl Conch, TrueForm 3, Petite Rose, Moon Horn,
    PAnther, Mecha, Lagoon 

# Bugfixes

- Device Fixes
  - New Satisfier Pro 2 Gen 3 identifier
  - New Svakom Vick Neo identifier
  - Correct Motorbunny rotation command

# 7.1.13 (2024-01-27)

## Features

- Added Device Support
  - metaXsire Upton, Tadpole
  - Svakom Pulse Galaxie
  - JoyHub VibSiren

## Bugfixes

- Warning log message on Svakom Sam known not to have speed control
- Added more logging to bluetooth writes and device init
  - In service of debugging issues with Lovense Solace w/ v31 firmware

# 7.1.12 (2024-01-21)

## Features

- Move from async-tungstenite to tokio-tungstenite
  - We only build for tokio at this point anyways (outside of wasm)
  - Easier to integrate with rustls, removes OpenSSL requirement on some platforms
- Added Device Support
  - Mysteryvibe Molto
  - Svakom Alberta, Ava Neo
  - MonsterPub Gen 2+ (multiple devices)
  - Joyhub Petalwish 2
  - metaXsire Nolan
  - Cooxer Bullet Vibe
  - iToys Seagull
  - Coleur Dor VX045A
  - Coleur Dor VX236A
  - Coleur Dor DT250A
  - Svakom Winni 2
  - Svakom Magic Suitcase
  - metaXsire Tay
  - Leten devices (mostly legacy now they seem to have moved to MuSe)
  - Lovense Ambi (new UUIDs)
  - Svakom Alex Neo 2 (new identifier)
  - JoyHub Vortex Tongue
  - JoyHub Velocity
  - JoyHub Rosella 2
  - Lelo Hugo 2
  - Svakom Magic Suitcase
  - OhMiBod Sphinx
  - MizzZee v3
  - Vibcrafter (multiple devices)

## Bugfixes

- Streamed JSON now handled by serializer
  - May fix issues with some older Buttplug C# programs.
- Lovense Lapis now controls all 3 vibrators using Lovense Connect
- Lovense Solace works via Lovense Connect
- Fix mode flags on Foreo devices
- Add new BLE name for Sinoli devices

# 7.1.11 (2023-11-18)

## Bugfixes

- Update to btleplug 0.11.3
  - Fix macOS missing import issue

# 7.1.10 (2023-11-18)

## Bugfixes

- Update to btleplug 0.11.2
  - Fixes issue with non-UTF-8 advertisement names in Android, which blocked Lovense Solace
    connection

# 7.1.9 (2023-11-15)

## Features

- Added Device Support
  - Lovense Solace (speed only, depth coming at some point in the future)
  - Satisfyer Pro Gen 3
  - OhMiBod Foxy, Chill

# 7.1.8 (2023-11-04)

## Features

- Added Device Support
  - Lovense Vulse, Lapis
  - Funtown Caleo, Jive

## Bugfixes

- Fix Svakom Sam 2 Detection
- Add missing Synchro identifier

# 7.1.7 (2023-10-19)

## Features

- Added Device Support
  - Mysteryvibe Crescendo 2, Tenuto 2
- Keepalive option added to btleplug, allows devices to resend commands to keep connections alive
  - Needed for iOS backgrounding

## Bugfixes

- Fixed issue with Svakom Sam Neo having different commands between same hardware

# 7.1.6 (2023-10-08)

## Features

- Added Device Support
  - Foreo - all vibrating products for brand
    - Yes, Buttplug now supports "beauty products"

## Bugfixes

- #588: Fix issue with lovense dongle support being broken in single threaded runtime situations
  (which includes intiface engine.)

# 7.1.5 (2023-09-23)

## Features

- Added Device Support
  - Magic Motion Solstice X, Zenith, Xone
  - Fredorch Rotary devices

# 7.1.4 (2023-09-16)

## Features

- Added Device Support
  - Mysteryvibe Tenuto Mini
  - Synchro Edge
  - Lovense Ridge
  - Kiiroo Fuse 1.1
  - Svakom Theodore, Barzillai, Mimiki, KyuKyu (BeYourLover), Tara X (ToyCod)

## Bugfixes

- Fix test executors

# 7.1.3 (2023-09-08)

## Features

- Added Device Support
  - Lastic
  - OhMiBod Esca (new identifier)
  - Hismith v4
  - Svakom Barnard (Fantasty Cup), Aravinda
  - Satisfyer Pro 2 Gen 3
  - Pink Punch Peachu
  - Eropair S1, V1
- Remove requirements for OpenSSL
  - Makes building dependent projects much simpler
- Make library compile cleanly under WASM again
- Start work to let library run in single threaded async contexts

## Bugfixes

- Update to btleplug 0.11.1
  - Might fix some issues with device disconnection on windows
- Fix issue with Svakom Neo disconnection
- Fix issues with missing includes/features for compiling dependent projects that don't
  use the client/server features

# 7.1.2 (2023-07-16)

## Features

- Added Device Support
  - Magic Motion Fugu2, Flamingo T

## Bugfixes

- Fix expectation of populated data field in Lovense Connect packet

# 7.1.1 (2023-07-09)

## Features

- Added Device Support
  - WeVibe Sync Lite
  - Long Lost Touch Possible Kiss
  - Lovense Exomoon
  - Kiiroo Realm 1.1
- A bunch of under-the-covers stuff to prepare Intiface Engine/Central for websocket device UX

## Bugfixes

- Update btleplug to v0.11, may fix some android bugs
- Fix Wevibe Chorus protocol impl
- Fix Hismith matching code
- Fix Roselex device characteristics
- Fix issues w/ websocket device ports not closing in some instances
- Fix issues w/ websocket device system dropping some messages due to being text vs binary

# 7.1.0 (2023-05-21)

## Breaking(ish) Changes

- ButtplugRemoteServer has been moved up to Intiface Engine, because it mostly deals with handling
  external connections, and was only used by the engine. While this is an API breaking change on the
  server side, I'm pretty sure no one ever used this outside of Intiface Engine, so I'm just making
  this a point release. I guess if someone did use it, I'm gonna hear about it.

## Features

- Added Device Support
  - Svakom Phoenix Neo 2, Hannes, Ella Neo, Edeny, Tammy Pro
  - WeVibe Sync 2
  - FitCute Kegel Rejuve
  - Lelo Tor 3
  - Auxfun Remote Control Box
  - Hismith Sinloli
  - Lovense Tenera
  - Xiuxiuda Devices
  - Kiiroo Pearl 2+
- Rework configuration system for easier loading of user device configs

## Bugfixes

- Fix name for Xibao Smart Masturbation Cup
- Fix Folove Telescopic Prostate Massager communications
- Fix Svakom Iker identification to use manufacturer value
- Fix protocol impl for some Kiiroo Vibrators
- Fix Vorze UFO TW Nipple Stimulator rotation commands
- User configs with non-null but unknown identifiers should use default configs
- Clarify error message for Websocket Server not being able to bind
- Close websocket device server when ping times out (won't show device as disconnected otherwise)

# 7.0.2 (2023-02-19)

## Features

- Added Device Support
  - Kizuna Smart
  - Svakom Phoenix Neo
  - Svakom Vivianna
  - Svakom Pulse Union
  - Sakuraneko Koikoi
  - Sakuraneko Nukunuku

# 7.0.1 (2023-01-16)

## Features

- Added Device Support
  - Galaku Nebula
  - Xibao Smart Masturbation Cup
  - Sensee Diandou Rabbit
  - Svakom Pulse Solo
  - Fox Device

## Bugfixes

- Remove duplicate WeVibe Chorus Definition
- Correct Folove device detection
- Correct attributes for Lelo F1 v1/v2

# 7.0.0 (2022-12-30)

## Breaking API Change

- In order to accommodate ScalarCmd flexibility, VibrateCommand in the ButtplugClient has been
  changed to ScalarValueCommand. This changes the signature of the vibrate method in the client,
  which may cause breakage.
- Removed IPC connectors. They were never really advertised as usable, and the longer we've stuck
  with websockets, the less feasible breaking out new connectors has become.

## Features

- Added ergonomic action methods to ButtplugClient (ability to query for attributes related to
  scalar actions, etc...)
- Added new_json_ws_client_connector() convenience method for creating new client connectors
- Added Device Support
  - GBalls v3
  - The Cowgirl/The Unicorn
- Removed old, unused rust book
- Moved spec to new docs site repo (https://docs.buttplug.io)

## Bugfixes

- Fixed issue where ButtplugClient could make ill-defined requests for device control. This was done
  by added finalizers to ButtplugMessage, so that DeviceAdded and DeviceList messages can include scalar/sensor indexes in their message attributes. This problem will be fixed at the spec level in spec v4.
- Fix buttplug device config schema issue blocking use of websocket devices.
- Fix Svakom Iker identifiers

# 6.3.0 (2022-12-19)

## Features

- Added Device Support
  - MetaXSire (all products)
  - Lovense Gemini, Gravity, Flexer
  - Roselex (all products)
  - Hismith Widolo devices
  - TryFun Yuan series devices
- Add support for the Kiiroo Pearl 2.1 Sensors and Battery Level

## Bugfixes

- #532: Simplify Generic Command Manager Match-all Processing
  - Fixes issues with Satisfyer/WeVibe/Magic Motion for applications with high thruput
- Fix issues with Lovense vibration command formation between single/multi vibrator devices
- Fix issue with the Vorze Cyclone SA not being addressed correctly
- Fix Hgod protocol update loop
- Fix deserialization of multi-type battery field in Lovense Connect service

# 6.2.2 (2022-11-27)

## Bugfixes

- #515: Define capable build platforms with Device Config Managers
  - Fixes issues with trying to build for platforms like mobile and desktop at the same time, since
    it's somehow 2022 and we still can't define per-platform crate features in cargo.
- #516: XInput protocol should return true for needs_full_command_set
- #517: Lovense Dongle should return Ok() on subscribe/unsubscribe instead of throwing
  unimplemented()
- #518: DeviceManager now stops hardware on shutdown
- Shorten "No Description" message to N/A
- Updated to btleplug that no longer has a common log message as Error level on CoreBluetooth
  (macOS/iOS)

# 6.2.1 (2022-11-24)

## Features

- Added Lovense Max Constriction capabilities for Lovense Connect Service users
- Added Device Support
  - Lovense Flexer (generic vibration only)
  - Hismith (Only available via Buttplug v3 ScalarCmd commands)

## Bugfixes

- Reduced polling frequency for Lovense Connect Service
- Remove Folove advertised services (fixes Hismith device finding)

# 6.2.0 (2022-11-05)

## Features

- Expose DeviceManager from server as an Arc (used for Server direct device access in Intiface Engine/Central)
- Added Device Support
  - WeVibe Chorus
  - Nobra BLE Controller
  - WeToy MiNa
  - Pink Punch Sunset Mushroom
  - Sakuraneko toys
  - Synchro
  - Lelo Tiani Harmony, Ida Wave

## Bugfixes

- Update to btleplug version that doesn't break Android
- #497: If websocket server gets a ping, return a pong
- Remove Battery/RSSI Messages from SpecV3 Union
- Only allow DeviceManager's shutdown() to be called from within ButtplugServer
- Fix issue with btleplug Adapter Event Loop stalling on exit
- Fix panic in Device Manager Event Loop on very early exit
- JSON Serializer should use V3 for decoding by default
- Set Device Manager gating so if it is ever shut down it won't come back
- Change Websocket server ping timing to once every 10s (was every 1s)
- Fix Write type for Lovense Desire devices
- Fix protocol issue in MagicMotion toys with multiple vibrators
- Fix Ankni/Roselex protocol handshake
- Add additional endpoints used by Ankni

# 6.1.0 (2022-10-15)

## Features

- Bluetooth devices can now be detected using manufacturer data in advertisements
  - Also fixes #462
  - This requires a major version change to the device config library, hence the minor version
    change.
- Add new Keon device name to device config

## Bugfixes

- #488: Fix issue where ServerInfo returning higher message version than RequestServerInfo can break
  older clients.
- #491: Implement disconnect on Drop for Bluetooth devices

# 6.0.1 (2022-09-24)

## Breaking Changes

- Act like these came in v6, I don't wanna rev to v7 already and afaik no one depends on this
  version of the library yet so y'all just get to deal.
- Device Configuration now uses major/minor semantic versions so we can identify when we can't load
  a device config file.

## Features

- #480: Add Major/Minor versions to device configs.
- Added Hardware Support:
  - Meese Tera
  - Hismith Thrusting Cup
  - Vorze UFO TW
  - Mizz Zee V2 Devices
  - MagicMotion Bobi
  - Satisfyer devices with > 2 vibrators
- Add ability to shutdown a device manager and disconnect all devices explicitly
  - Needed for mobile apps
- Update to btleplug v0.10.1, fixing disconnect on macOS/iOS

## Bugfixes

- #462: Check advertisements to see whether we even have any matching configurations before trying
  to connect to a device.
- #483: Move Lovense Connect Server checking to every 5 seconds, which will hopefully stop rate
  limiting
- #481: Fix issue with device creation tracing span being applied to incorrect tasks
- Correct timing issue in Fredorch protocol
- #479: Make Serial and XInput DCMs less log spammy during scans
  - Serial is backed off to scanning once per 10s, XInput now only loads the XInput library on
    initial construction.

# 6.0.0 (2022-08-29)

## Breaking Changes

- `connector` module moved to `core` instead of top level.
- Renamed `messages` module to `message` (to stay with singular style module naming).
- `device` module now split between `core` (Endpoints struct now in `message` module) and `server`
  (impl, protocols, configs, everything that is server specific now lives there).
- `DeviceImpl` renamed to `Hardware` to clearly signify that it is how we actually communicate with
  hardware (real or virtual).
- `ButtplugDevice` renamed to `ServerDevice`, to denote that it's the representation of the device
  in the server, now clearly separate from `ClientDevice`.
  - Gosh I am really bad at naming things.
- Device configuration file format changed to work with new format of DeviceAdded/DeviceList
  messages. Also removed language specifier for device names, as these were never actually
  used.
- User Device Configuration File format completely overhauled to handle device specifiers (easier
  way to identify unique devices).
- In-process client creation utility method moved to util module.
- Buttplug Server and its components are now constructable via builders, and are immutable after
  construction. This makes management and additions far easier to reason about, and there was no
  reason for mutability there anyways.
- Running StartScanning when a scan is already running no longer throws an error.
- Except in some special cases (WebBluetooth, mostly), device scans will now run until StopScanning
  is sent. Waiting for the ScanningFinished event is no longer recommended on platforms without these special needs.
- GenericCommandManager is now internally mutable, simplifying borrow handling.
- Removed "connected()" status getter from Hardware implementations. We assume that, if a device
  instance is alive, it's connected. Otherwise the device manager will have dropped it. This assumption was made in earlier versions of the library because this was never used, it is now just being made explicit.
- Protocol handlers completely rewritten to minimize amount of code handling. Protocol handlers
  should now handle device identification, initialization, and simple command handling, with all other management (generic command caching, etc) handled in owner structs or traits above the handler itself.
- Replace pub struct members with getset calls.

## Features

- Overhauled device configuration system so it can de/serialize and handle user configuration
  stacking. This is important for being able to load, edit, and save configs from outside the
  library, in applications like Intiface Desktop.
- Simplified the device creation system, making tracing how a device goes from advertisement to
  usable device somewhat clearer (but it's still complicated af).
- Added TimedRetryHardwareCommunicationManager wrapper, for generic retry handling of hot plugging
  in comm managers that don't constantly scan (XInput, USB, etc...).
- Added ScalarCmd message
  - These will replace messages that take a single scalar parameter, specifically VibrateCmd.
    ScalarCmd adds an extra attribute, called _ActuatorType_, that denotes what the scalar commands
    affects when it is sent. For instance, with vibrators the actuator type is _Vibrate_, for
    flywheel fucking machines (hismith, lovense, etc) and some strokers it's _Oscillate_, for the
    Lovense Max air bladder it's _Constrict_, etc... This allows us to add new simple acutation
    types via types of ScalarCmd instead of having to add a new message to the protocol for every
    type.
- Added SensorReadCmd/SensorSubscribeCmd/SensorUnsubscribeCmd/SensorReading messages
  - Allows Buttplug to take input from devices, instead of just sending them commands. Replaces
    BatteryLevelCmd/RSSILevelCmd currently, but also adds the ability to read other sensors like
    buttons, pressure sensors, etc.
- Added Hardware Support
  - KGoal Boost
  - Hismith Fucking Machines
  - XInput Battery Levels
  - Lovense Max Air Bladder (via ScalarCmd Constrict Actuator Type)
- New scriptable test system for end-to-end (virtual) device testing, across v2 and v3 of the
  Buttplug Protocol (v0/v1 coming in a later update).
- Device configurations now specify a step range instead of step count. This allows users to
  customize the range of values a device can take, for instance setting a maximum speed that
  Buttplug will run a device at. Clients are still given Step Count for the number of states
  available for a message. For instance, if a device has a normal range of [0, 10], a client would
  get a step count of 10. However if a device has a range of [0, 5], the client would only see a
  step count of 5. Changing the bottom of the range will allow the user to set a lower bound as well
  as an upper bound, which is useful for linear devices to isolate a stroking range.
- Added `FeatureDescriptor` to device features. This will describe what certain features of devices
  are, relating to their available device commands. For instance, we can denote which Lovense Edge
  vibrator is the insertable vibrator versus which is the perineum vibrator, and that information is
  sent to the client.

## Bugfixes

- Fixed issue with collisions for devices that don't advertise enough information in their
  bluetooth advertisement (namely, Satisfyer and Magic Motion devices).
- Rebuilt Buttplug JSON Schema to handle all message spec versions simultaneously yet clearly
  (versus the vague, underspecified mess it was before).
- Ping timeouts now actually stop devices
- Close the server side of a websocket when the client side closes (this was causing issues with
  websocket tests on macos/linux, we can run those tests now!)


# 5.1.10 (2022-05-07)

## Bugfixes

- Fix issue with invalid configuration of certain Satisfyer devices that could cause crashes

# 5.1.9 (2022-04-26)

## Features

- Added MagicMotion Crystal Support

## Bugfixes

- Fixed issue with connection timeouts on Satisfyer Plugalicious
- Use product IDs for identifying Satisfyer toys

# 5.1.8 (2022-03-05)

## Features

- Update to blteplug v0.9.2
  - Should fix many issues with Windows Bluetooth, including panics in older versions of windows and
    the ability to disconnect devices
- New Hardware Support
  - Lelo F1S v2
  - OhMiBod NEX|3
  - Magic Motion Bunny, Sundae, Kegel Coach, Lotos, Nyx, Umi

## Bugfixes

- Fix Libo Elle protocol issues
- Fix Mannuo advertisement names

# 5.1.7 (2022-01-24)

## Features

- New Hardware Support
  - Lovense Calor
  - Hismith (REQUIRES EXTRA MODIFICATIONS CURRENTLY, see info at https://how.do.i.get.buttplug.in)
  - Folove Devices
  - Satisfyer Little Secret
  - WeVibe Nova (Alternative names)
- Add Buttplug Passthru Protocol (mostly for simulator development)

## Bugfixes

- Remove all unsafe calls (Didn't need them anyways)
  - Required switching from valico to jsonschema for schema validators

# 5.1.6 (2022-01-01)

## Features

- Add more log messages on host machine and configuration loading, helpful for debugging
- XInput controller names now also have their index (i.e. XInput Gamepad 1/2/3/4 versus just XInput
  Gamepad)

## Bugfixes

- #420: Fix issue with serial port DCM blocking itself when waiting for reads with no data
- #418: No longer error when user config file version not >= current device file version
- #417: Fix Lovense Connect JSON endpoint types to be more flexible
- #416: Warn when bluetooth and lovense dongles are both active
- Reduce severity of some common log messages from error to warn

# 5.1.5 (2021-12-18)

## Bugfixes

- Fix issue with Bluetooth scanning reporting finished status before it's actually done
- Add heartbeat for Satisfyer devices so they don't disconnect when no messages are sent
- Fix name/identifier handling for Satisfyer devices

# 5.1.4 (2021-12-08)

## Bugfixes

- WASM API updates (hadn't built WASM in a while, broke stuff :( ))

# 5.1.3 (2021-12-08)

## Bugfixes

- Fix missing tokio feature for Named Pipes

# 5.1.2 (2021-12-04)

## Bugfixes

- #413: Fix race condition with bluetooth advertisements causing multiple connection attempts to the
  same device simultaneously.

# 5.1.1 (2021-12-03)

## Bugfixes

- #410: Fixed issue with hidapi on linux not working with Lovense dongles
- #411: Fixed issue with bluetooth devices without full advertisement info being ignored
- #412: Added more bluetooth manufacturer identifier data

# 5.1.0 (2021-12-01)

## Features

- Update to Rust Edition 2021
- Added Named Pipe/Unix Domain Socket connector
- Updated to btleplug v0.9, simplified Linux bluetooth code to be in line with rest of platforms
- Bring User Device Configuration in-line with Device Configuration
- btleplug DCM now prints radio manufacturer to logs (useful for debugging)
- btleplug DCM can now identify devices via advertised services (useful for Satisfyer, Vibratissimo)
- Added Hardware Support:
  - Lovense Gush, Hyphy
  - LoveDistance devices
  - Satisfyer devices (Only works w/ CSR Bluetooth dongle, and only one device bonded at a time)
  - Hot Octopuss Pulse (Kiiroo/Bluetooth Edition)
  - Svakom Sam, Alex, Iker
  - ManNuo devices

## Bugfixes

- Renamed Lovense Quake to Lovense Dolce, Lovense Blast to Lovense Ridge
- Lovense Connect will now use user's IP instead of lovense.club resolution, bypassing issues with
  Lovense's DNS resolver
- Removed all .unwrap()s from library, either by cleanup or conversion to .expect() with useful
  message. This will probably fix multiple issues.

# 5.0.1 (2021-09-11)

## Features

- Added Hardware Support:
  - Lovense Quake
  - HTK Breast Massager
  - Ankni Candy
  - Hgod Butterfly Love

## Bugfixes

- Fix issue with FleshlightLaunchFW12Cmd support for TCode devices
- Fix issue with WeVibe using incorrect write types for bluetooth toys

# 5.0.0 (2021-08-07)

## Features

- Update to btleplug v0.8
  - Adds async API
  - 50% reduction in bluetooth code
  - Far more stable/reliably on macOS
  - Paves the way for Buttplug Android
- Added Websocket Server Communication Manager
  - Paves the way for Device Simulators
  - Makes DIY devices much easier to connect for prototyping
- Added Device Allow/Deny capabilities
  - Allows users ability to never or always connect to certain devices
- Added Device Configuration addition capabilities
  - While loading a default device configuration is still possible, we now externalize the loading
    functions so that the library isn't bound to certain configuration formats.

## Bugfixes

- #381: Fix issue with DNS resolution for Lovense Connect Service

## Breaking Changes

- No breaking Client API changes
- Server API Surface changes
  - In lieu of proxying APIs up thru the server, we now just expose a way to get a reference to a
    device manager for adding protocols, configurations, etc...

# 4.0.4 (2021-07-04)

## Features

- Added Hardware Support
  - Adult Festa Rocket+1D
  - Vorze Piston SA
  - Patoo Carrot/Vibrator/Devil
  - TCode v0.3 Devices
    - Currently only supports L0 Up/Down Axis. More command support coming soon.

## Bugfixes

- Fix bounds for FleshlightLaunchFW12Cmd (Used by legacy programs)
- Change panic to error return when websocket server can't bind to port
- Throw error if first message received by a server is invalid (not spec version)

# 4.0.3 (2021-06-18)

## Bugfixes

- #346 - Fix variations in type returns from the Lovense Connect API when using iOS/Android

# 4.0.2 (2021-06-11)

## Bugfixes

- Implement workaround for Intiface Desktop engine stall issues, via changing log message levels on
  messages fired from threads.

# 4.0.1 (2021-05-31)

## Features

- Added hardware support:
  - Lovenuts brand toys
  - Svakom Neo vibrators
  - Je Jour Nuo and Dua

## Bugfixes

- Fixed toy addressing when using Lovense Connect Service
- Fixed issue with websocket writer stream not closing correctly when websocket is dropped.

# 4.0.0 (2021-05-02)

## Features

- #293: Protocols can now be added/removed to the system dynamically
  - This allows addition of outside protocols, instead of having to build everything into the
    library. Also allows for protocols to be removed to fine tune library usage.
- #320: Device Communication Managers now use a builder pattern for creation.
  - This allows addition of extra parameters when creating Device Comm Managers, which will be
    useful for creating Comm Managers on top of online services that require authentication.
- #319: Lovense Connect Application support
  - Allows users to connect to Lovense devices via a local HTTP connection to their phone.
- Added Hardware Support
  - GBalls 2
  - Femtometer Lilac
  - Cachito Tao
  - Cachito Ice Cream

## Bugfixes

- #315: Device command not handled message now lists which command type wasn't handled
- #316: Lovense dongle now restarts scan if scan is stopped by dongle before request

## Breaking Changes

- Signature for ButtplugServer::add_comm_manager changed
- Signatures/traits for all Device Comm Managers changed

# 3.0.3 (2021-04-24)

## Bugfixes

- Fix RawWriteCmd JSON schema to handle WriteWithResponse field

# 3.0.2 (2021-04-22)

## Bugfixes

- Device Config File update to fix a crash in MagicMotion v2 and v3 battery queries.

# 3.0.1 (2021-04-18)

## Bugfixes

- #313: Fix reading of lovense battery status when device is running
- Don't try to compile native-tls when not using websockets feature

# 3.0.0 (2021-04-11)

## Features

- Added hardware support:
  - Lovelife (OhMiBod) Lumen
  - Mysteryvibe Poco
  - Libo Selina
- #311: Reduce runtime support to simplify library
  - Added tokio runtime support
    - Needed for Unity support of buttplug-rs.
  - Removed async-std runtime support
    - We don't really have the resources to keep supporting multiple runtimes, and async-std has
      tokio compat.
  - Removed futures::ThreadPool runtime support
    - This was easy to support when the library started, but seems silly now.
- #310: Removed Secure Sockets as on option on Websocket Servers
  - Our original reason for implementing this was for browsers that didn't handle connecting to
    mixed content (i.e. https website connecting via websockets to http localhost). Firefox resolved
    this a few months ago, and Chrome and Edge both support it too. Safari has always been a weird
    mess, so we don't really care there. If users still need this functionality, they can set up their own reverse proxy, but this frees us from having to support this for them, which took a ton of code and time.
- Added more logging, now using fields to track device lifetimes across log contexts
- #312: Added more Drop implementations to make clean shutdown happen properly

## Bugfixes

- #295: Prettylove devices require WriteWithResponse to control properly.

## Breaking Changes

- ButtplugWebsocketServerConnectorOptions no longer has secure options, and the insecure port option
  is now mandatory.
- Removed build features for no longer supported runtimes, native programs using Buttplug as an
  executable will need to spin up a tokio runtime, either via the tokio::main macro or manually.

# 2.1.9 (2021-04-04)

## Bugfixes

- #305: Updated btleplug to not panic on read/write failures on Windows UWP, which may fix some
  issues with bluetooth disconnections.
- #302: Lovense devices no longer stall forever when device disconnects while reading battery
- #300: Error logs changed to warn when bluetooth dongle not present
- #299: Lovense dongle state machine now handles more state/status transfers, resulting in less
  error messages.

# 2.1.8 (2021-03-25)

## Bugfixes

- #296: Fix issue with bluetooth devices not registering disconnects correctly on some instances in
  windows.

# 2.1.7 (2021-03-08)

## Features

- Added LoveHoney Desire Egg hardware support
- Handy now supports FleshlightLaunchFW12Cmd when using <= v1 of the protocol, meaning it will work
  with ScriptPlayer and Syncydink
- Vorze now supports VorzeA10CycloneCmd when using <= v1 of the protocol, meaning it will work with
  ScriptPlayer and Syncydink

# Bugfixes

- #281: Split the Kiiroo v2.1 protocols into toys that require init, versus those that don't.

# 2.1.6 (2021-02-28)

## Features

- Update to btleplug v0.7
  - Mostly bugfixes and cleanup
- Add device support for the Lovehoney Desire Love Egg

## Bugfixes

- Remove info level message that fires on every btle characteristic notification.

# 2.1.5 (2021-02-20)

## Bugfixes

- #283: Fix timing to connection status update in Client, reducing the possibility of races with the
  server handshake.
- #284: Fix issue where devices can cause panics if they disconnect in
  ButtplugProtocol::initialize()

# 2.1.4 (2021-02-13)

## Features

- Additional Hardware Support
  - The Handy

## Bugfixes

- #280 - Lovense Serial Dongle had some timing issues that caused a thread panic
- #277 - Add retry loop for communicating with LoveAi Dolp and other toys 

# 2.1.3 (2021-02-10)

## Bugfixes

- #279: Allow device command arguments to be set to 1.0 without trigger validation errors.

# 2.1.2 (2021-02-07)

## Bugfixes

- #276: Fix StopAllDevices default id so it will still send in in-process situations.

# 2.1.1 (2021-02-06)

## Bugfixes

- Fix compilation issue with btleplug in Linux and on CI
- Fix device enumeration example to use all comm managers by default

# 2.1.0 (2021-02-04)

## Features

- Hardware Support
  - Nobra's Silicone Dreams (All bluetooth 2/RFCOMM controllable toys)
  - Lovense Diamo
- Add message validation 
  - We were only checking messages via the JSON schema, which wasn't handling a lot of invalid
    content. Things should now throw more useful errors on invalid content.
  - Outside of message ids, messages are now immutable, meaning we don't have to check validity
    multiple times. 
- Clean up connector and transport code
  - Fix a lot of weird return types and trait bounds.
- Add more tests, as well as testing capabilities
  - Thanks to the connector/transport cleanup we can now test connectors and serializers easier.
- Consolidate support repos into main repo
  - buttplug-rs now has both device config and schema repos in it.

## Bugfixes

- #265: Drop devices that are connecting when there is no device manager

# 2.0.6 (2021-01-26)

## Bugfixes

- #261: Fix crates URL in README
- #262: Client device status nows updates to disconnected on client disconnect
- #263: Client now clears held devices on disconnect
- #264: System no longer panics on client events when client event stream has no listeners

# 2.0.5 (2021-01-24)

## Bugfixes

- #260: original_device_messages shouldn't be deserialized.
  - Put the wrong decorator on the field. :(

# 2.0.4 (2021-01-24)

## Bugfixes

- #258: XInput devices no longer cause panic on discovery due to address/identifier mismatches
- #259: Make sure futures-timer works with WASM
  - Also fixes #253 (Onyx+ issues) because I can readd the init delay between packets for WASM

# 2.0.3 (2021-01-21)

## Bugfixes

- #256: Constrain device message types available in clients
  - Clients shouldn't have to deal with deprecated messages, but sometimes the server will send them
    in attributes (See #257). Constraint the available messages to those currently live in the spec.
    Only really affects FFI libraries.

# 2.0.2 (2021-01-18)

## Bugfixes

- Fix (another) issue with Lovense dongle device scanning not handling state updates correctly.

# 2.0.1 (2021-01-18)

## Bugfixes

- Update to btleplug 0.5.5, fixing issue with async-std channel API versioning
- Fix issue where not having a Lovense Dongle causes the device scanning system to hang forever

# 2.0.0 (2021-01-18)

## Features

- #202, #227, #228, #246: Rebuild the event and channel systems using tokio channels
  - Switched from using async-channel and the broadcaster crate to tokio's sync module, which has a
    far more ergonomic channel system. Greatly reduces chances of leaking channels.
  - Massively changes how the surface API looks, hence rolling major versions.
- Restructed DeviceImpl objects to reduce boilerplate and centralize check logic.
- Restructed internal event loops to be slightly less messy.
- Added device support
  - Lovense Ferri

## Bugfixes

- #254: Fix issue with devices connected to Lovense Dongle not being picked up on startup
- #250: Websocket server no longer crashes when started without ports

# 1.0.5 (2021-01-09)

## Features

- #242: Added Libo support

## Bugfixes

- #244: Prettylove protocol handling now uses command caching, fixed name lookup
- #245: Fix device configuration for Virtual Rabbit

# 1.0.4 (2021-01-02)

## Features

- #238: XInput gamepads now fire Disconnection events, which should stop devices from being
  double-added.

# 1.0.3 (2021-01-01)

## Features

- #235: XInput now rescans every 1 second until told to stop (versus scanning once and never
  scanning until StartScanning is called again.)
- #231: Use LTO in release builds

## Bugfixes

- #236: Bluetooth device scanning no longer ignores addresses without names in advertisements. Fixes
  issues where RSSI updates were received before advertisements.
- #234: Fix declaration of Send/Sync on ButtplugFutureSharedState

# 1.0.2 (2020-12-31)

## Bugfixes

- Fix race condition that caused ScanningFinished to be fired before some managers may have finished
  scanning.
- Fix a bunch of intermittent test issues that were plaguing CI.

# 1.0.1 (2020-12-27)

## Bugfixes

- Roll back Valico dependency to 3.4.0. 3.5.0 brings in a ton of extremely old and unmaintained
  libraries, some of which break WASM.

# 1.0.0 (2020-12-27)

## API Changes

- Return futures::Stream instead of futures::StreamExt for event streams
- Add device config file version loading

# 0.11.3 (2020-12-22)

## Bugfixes

- Fixed memory/task leak and CPU spikes when start scanning was called often. (#226)
- Fixed possible race due to lack of trait bounds on future types (#225)

# 0.11.2 (2020-12-12)

## Bugfixes

- Client now emits ServerDisconnect when server disconnects.
- Client and Server now emit ScanningFinished when all scanning has stopped.
- Adjust Send trait requirements for async spawned tasks from the WASM manager.
- Remove unused WASM bindings.

# 0.11.1 (2020-11-26)

## API Changes

- Add hardware support
  - Mysteryvibe (all products)
- Implement device disconnect in buttplug-rs client
- Implement manual ping in buttplug-rs client

## Bugfixes

- Remove Default trait implementations for device messages
  - They always need device IDs and shouldn't be default constructable.
  - Fixes StopDeviceCmd issues in buttplug-rs client

# 0.11.0 (2020-10-31)

## API Changes

- ButtplugClientDevice now sent with ButtplugClientEvent::DeviceRemoved event
  - Used to just send the index, but that's annoying for the user.
- More WASM type exposure.

# 0.10.1 (2020-10-24)

## Bugfixes

- Devices now keep their indexes for the life of the process, based on device
  address.
- Fixed a lot of Lovense Dongle bugs
  - Devices now register disconnects
  - Dongle now emits scanning finished events
  - Dongle now handles being unplugged (but not replugged yet)
- Removed .unwrap()s in BTLEPlug that were causing crashes.

## API Changes

- Exposed Endpoint enum to WASM when doing wasm runtime builds

# 0.10.0 (2020-10-17)

## Features

- New Device Support
  - Kiiroo Keon
- Added Raw commands, which allows direct read/write access to devices.
  - Must be explicitly turned on during server creation
  - Devices will have "(Raw)" appended to their display names to let users know
    raw commands are active.
- Added 32 generic endpoints
  - Needed for Raw message setup
- Added stop_all_devices to client API

## Bugfixes

- Websocket Server no longer panics on bogus connect (TLS on non-TLS port, vice
  versa, browser hasn't accepted cert, etc...)
- Server now stops scanning when a client disconnects

## API Changes

- Removed RequestLog/Log access in API, as those messages were deprecated in v2
  of the message spec.
  - Older apps will just receive an error when RequestLog is sent.
- Creating a server now takes a Server Options struct argument.
  - All methods of creating servers (directly, ButtplugRemoteServer, servers in
    InProcessConnectors) have been updated to this format.
- DeviceConfigurationManager no longer static
  - Makes it easier to configure for Raw Messages, and is also just a better
    architecture in general.
- Fallible/non-Self-returning new() methods no longer called new()
  - Idiomatic rust requires infallible new() -> Self, and a lot of our
    constructors are fallible and sometimes return tuples.
- Removed ButtplugProtocolCreator
  - Was needed when we were using async_trait due to associated trait methods,
    now just bound on Self: Sized.

# 0.9.2 (2020-10-06)

## Bugfixes

- Update to btleplug 0.5.4, fixing a bug with trying to read from bluetooth
  devices on macOS.

# 0.9.1 (2020-10-05)

## Bugfixes

- Update to btleplug 0.5.3, fixing a bug with trying to write to bluetooth
  devices on macOS.

# 0.9.0 (2020-10-04)

## Features

- Implements BatteryLevelCmd/BatteryLevelReading
  - Can read battery values from Lovense and some MagicMotion devices.

## Bugfixes

- Devices now only pay attention to their events, not all events ever.
  - Devices were mistaking other devices disconnecting for their own
    disconnection, causing tons of issues in 2+ device situations.

# 0.8.4 (2020-10-01)

## Features

- Added more Lovense UUIDs
  - Should work with Ambi now? Maybe?
  - Might work with Lovense Mission
- Add wasm-bindgen async manager (Needed for WASM FFI)
- Implement read capabilties for btleplug comm manager devices

## Bugfixes

- Fixed LeloF1s bringup
- Fixed Onyx+ connection creation (requires pairing with OS)
- Fixed WeVibe initial connection (now vibrates on pair)
- EventReceiver is now StreamExt instead of SinkExt (fixed for FFI)
- Fix device message enum ordering (needed for test verifications)
- Make sure we send StopDeviceCmd in device attributes
- Fix issues with Youou and wildcard name lookups

# 0.8.3 (2020-09-20)

## Features

- Add wasm-bindgen executor to async-manager
- Add serializer type default to RemoteClientConnector type to simplify
  definitions.

## Bugfixes

- Expose Connector building traits to public API surface
- Add FleshlightLaunchFW12 message capabilities to device config

# 0.8.2 (2020-09-13)

## Bugfixes

- Fix default feature list in Cargo that got changed while testing 0.8.1 :(

# 0.8.1 (2020-09-13)

## Bugfixes

- Fix compile issues missed due to feature build failure
  - Need to get feature building into CI. :(

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
