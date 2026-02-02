# Buttplug

[![Patreon donate button](https://img.shields.io/badge/patreon-donate-yellow.svg)](https://www.patreon.com/qdot)
[![Github donate button](https://img.shields.io/badge/github-donate-ff69b4.svg)](https://www.github.com/sponsors/qdot)
[![Discourse Forums](https://img.shields.io/discourse/status?label=buttplug.io%20forums&server=https%3A%2F%2Fdiscuss.buttplug.io)](https://discuss.buttplug.io)
[![Discord](https://img.shields.io/discord/353303527587708932.svg?logo=discord)](https://discord.buttplug.io)
[![bluesky](https://img.shields.io/bluesky/followers/buttplug.io)](https://bsky.app/profile/buttplug.io)

[![Crates.io Version](https://img.shields.io/crates/v/buttplug)](https://crates.io/crates/buttplug)
[![Crates.io Downloads](https://img.shields.io/crates/d/buttplug)](https://crates.io/crates/buttplug)
[![Crates.io License](https://img.shields.io/crates/l/buttplug)](https://crates.io/crates/buttplug)

<div align="center">
  <h3>
    <a href="https://docs.rs/buttplug">
      Rust API Documentation
    </a>
    <span> | </span>
    <a href="https://buttplug.io/docs/dev-guide">
      Developer Guide
    </a>
    <span> | </span>    
    <a href="https://buttplug.io/docs/spec">
      Protocol Spec
    </a>
    <span> | </span>
    <a href="https://awesome.buttplug.io">
      Apps and Games List
    </a>
  </h3>
</div>

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: light)" srcset="images/buttplug_rust_docs.png">
    <source media="(prefers-color-scheme: dark)" srcset="images/buttplug_rust_docs_light.png">
    <img src="https://raw.githubusercontent.com/buttplugio/buttplug/master/images/buttplug_rust_docs.png">
  </picture>
</p>

## Are you in the right place?

If you're just looking to hook your hardware up to something that says it "supports Buttplug/Intiface" (like one of the many apps/games in [our awesome list](https://awesome.buttplug.io)), you're in the wrong place. For that you'll most likely want to go check out [Intiface Central](https://intiface.com/central). 

This is where we store all of the source code for the libraries that run your hardware. If you're a developer and interesting in that sort of thing, read on...

## Introduction

[Buttplug](https://buttplug.io) is a framework for hooking up hardware to interfaces, where:

- hardware usually means sex toys, but could honestly be just about anything
- interfaces usually means media players or games, but could also be just about anything

It's basically a userland HID manager for things that may not specifically support formal HID.

In more concrete terms, think of Buttplug as something like [osculator](http://www.osculator.net/)
or [VRPN](https://vrpn.github.io), but for sex toys. Instead of wiimotes and control surfaces, we
interface with vibrators, strokers, fucking machines, and other [hardware that can communicate with computers](https://iostindex.com) (though we do actually support vibration in gamepads, joycons, and more).

This repo contains all of the core libraries for the framework, as well as Intiface Engine, the command line utility for setting up Buttplug Servers.

[Intiface Central](https://intiface.com/central) is recommended for end users. It is a Flutter based GUI on top of Buttplug and Intiface Engine that runs on all desktop and popular mobile platforms.

We also produce [btleplug](https://github.com/deviceplug/btleplug), the host-side Bluetooth LE library that Buttplug uses to communicate with BLE devices.

## Apps, Games, and More!

For a list of applications using Buttplug and Intiface, see the [awesome-buttplug repo](https://github.com/buttplugio/awesome-buttplug).

## Hardware Support

Buttplug is currently capable of controlling toys via:

- Bluetooth LE (Desktop and Android/iOS)
- Serial Ports (Desktop)
- USB HID (Desktop)
- Lovense Devices via the Lovense Dongle (HID and Serial dongles, Desktop)
- Lovense Connect App (Desktop and Android/iOS)
- Websockets (for simulated and DIY devices, Desktop and Android/iOS)
- XInput gamepads (Windows only)

See [IOSTIndex](https://iostindex.com) for a full list of supported hardware (Filter on "Buttplug Rust").

## Documentation and Examples

To learn how to use the Buttplug Library, check out the [Buttplug Developer Guide](https://docs.buttplug.io/docs/dev-guide).

Examples are included in the [examples](examples/) portion of this repo.

## Crates

This project consists of the following crates:

| Crate Name | Description |
| ---- | ----------- |
| [buttplug](crates/buttplug) | meta-crate that's just a rehost on buttplug_client, see README for more info |
| [buttplug_client](crates/buttplug_client/) | Buttplug Rust Client, useful for building application that will access Intiface Engine or Intiface Central. We recommend most developers use this. See crate README for more info. |
| [buttplug_client_in_process](crates/buttplug_client_in_process/) | Buttplug Rust Client w/ integrated Buttplug Server. Useful for building standalone applications and examples. | 
| [buttplug_core](crates/buttplug_core) | Contains the protocol message schema, message class implementations, and structures shared by the client and server implementations. Will be rarely needed as a direct dependency. |
| [buttplug_server](crates/buttplug_server/) | The core server implementation, including server and device structures, all protocol implementations, etc... |
| [buttplug_server_device_config](crates/buttplug_server_device_config/) | Device configuration file loading and database implementation. |
| [buttplug_server_hwmgr_btleplug](crates/buttplug_server_hwmgr_btleplug/) | Bluetooth LE device communication support |
| [buttplug_server_hwmgr_hid](crates/buttplug_server_hwmgr_hid/) | HID device communication support |
| [buttplug_server_hwmgr_lovense_connect](crates/buttplug_server_hwmgr_lovense_connect/) | Lovense Connect device communication support (soon to be deprecated) |
| [buttplug_server_hwmgr_lovense_dongle](crates/buttplug_server_hwmgr_lovense_dongle/) | Lovense Dongle device communication support (soon to be deprecated) |
| [buttplug_server_hwmgr_serial](crates/buttplug_server_hwmgr_serial/) | Serial device communication support |
| [buttplug_server_hwmgr_websocket](crates/buttplug_server_hwmgr_websocket/) | Websocket device communication suppor, used for devices that may connect in ways not directly supported by other formats |
| [buttplug_server_hwmgr_xinput](crates/buttplug_server_hwmgr_xinput/) | XInput gamepad support (windows only) |
| [buttplug_tests](crates/buttplug_tests/) | For tests that need the whole framework |
| [buttplug_transport_websocket_tungstenite](crates/buttplug_transport_websocket_tungstenite/) | Communications transport for clients/servers using tokio-tungstenite |
| [intiface_engine](crates/intiface_engine/) | Command line interface for running a Buttplug server |

For more information on each crate, check the README in its directory in this repo.

## Compiling

On Windows and macOS, running `cargo build` should suffice for building the project. All
dependencies are vendored in.

On Linux, the following packages will be needed to build with all crates:

- `libudev-dev` (Required for serial port/HID support)
- `libusb-1.0-0-dev` (Required for serial port/HID support)

The package names are listed as their Debian requirements, and may be different for other
distributions.

## Other Language Implementations

See the [awesome-buttplug repo](https://github.com/buttplugio/awesome-buttplug#development-and-libraries) for a full list of client implementations in other programming languages.

## Filing Issues and Contributing

If you have issues or feature requests, please feel free to [file an
issue](https://github.com/buttplugio/buttplug/issues).

**We are not looking for unsolicited code contributions or pull requests, and will not accept
pull requests that do not have a matching issue where the matter was previously discussed in an issue on this repo or in one of our communication channels, listed below.** 

Pull requests should only be submitted after talking to [qdot](https://github.com/qdot) via issues
(or on [Discord](https://discord.buttplug.io), [our forums](https://discuss.buttplug.io), or via DMs
on one of our social media accounts if you would like to stay anonymous and out of recorded info on
the repo) and receiving approval to develop code based on an issue. Any random or non-issue pull
requests will most likely be closed without merging.

If you'd like to contribute in a non-technical way, we need money to keep up with supporting the
latest and greatest hardware. We have multiple ways to donate!

- [Patreon](https://patreon.com/qdot)
- [Github Sponsors](https://github.com/sponsors/qdot)
- [Ko-Fi](https://ko-fi.com/qdot76367)

## Inclusion of LLM Generated Code

Buttplug, Intiface, and related projects may contain LLM generated ("agentic") code. All code is reviewed by a developer, no vibecoding allowed. Agentic coding is mostly used for refactors and cleanup. We will do our best to call out where this happens, usually in commit messages.

## License

Buttplug is BSD 3-Clause licensed.

```text

Copyright (c) 2016-2026, Nonpolynomial, LLC
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

* Redistributions of source code must retain the above copyright notice, this
  list of conditions and the following disclaimer.

* Redistributions in binary form must reproduce the above copyright notice,
  this list of conditions and the following disclaimer in the documentation
  and/or other materials provided with the distribution.

* Neither the name of buttplug nor the names of its
  contributors may be used to endorse or promote products derived from
  this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

