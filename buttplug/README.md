# Buttplug (Rust Implementation)

[![Patreon donate button](https://img.shields.io/badge/patreon-donate-yellow.svg)](https://www.patreon.com/qdot)
[![Github donate button](https://img.shields.io/badge/github-donate-ff69b4.svg)](https://www.github.com/sponsors/qdot)
[![Discourse Forum](https://img.shields.io/badge/discourse-forum-blue.svg)](https://metafetish.club)
[![Discord](https://img.shields.io/discord/353303527587708932.svg?logo=discord)](https://discord.buttplug.io)
[![Twitter](https://img.shields.io/twitter/follow/buttplugio.svg?style=social&logo=twitter)](https://twitter.com/buttplugio)

[![Crates.io Version](https://img.shields.io/crates/v/buttplug)](https://crates.io/crates/buttplug)
[![Crates.io Downloads](https://img.shields.io/crates/d/buttplug)](https://crates.io/crates/buttplug)
[![Crates.io License](https://img.shields.io/crates/l/buttplug)](https://crates.io/crates/buttplug)

<div align="center">
  <h3>
    <a href="https://docs.rs/buttplug">
      API Documentation
    </a>
    <span> | </span>
    <a href="https://buttplug-spec.docs.buttplug.io">
      Protocol Spec
    </a>
    <span> | </span>
    <a href="https://buttplug-developer-guide.docs.buttplug.io">
      Developer Guide
    </a>
    <span> | </span>
    <a href="https://github.com/buttplugio/buttplug-rs/releases">
      Releases
    </a>
  </h3>
</div>

<p align="center">
  <img src="https://raw.githubusercontent.com/buttplugio/buttplug-rs/dev/buttplug/docs/buttplug_rust_docs.png">
</p>

Rust implementation of the Buttplug Intimate Hardware Protocol,
including implementations of the client and, at some point, server.

## Read Me First!

If you are new to Buttplug, you most likely want to start with the [Buttplug
Website](https://buttplug.io) or the [Buttplug Core Repo](https://github.com/buttplugio/buttplug).

For a demo of what this framework can do, [check out this demo
video](https://www.youtube.com/watch?v=RXD76g5fias).

Buttplug-rs is a full fledged implementation of Buttplug, implementing both the client and server
portions of the system. Implementations for other langauges (such as C# and JS) are built on top of
the Rust library. See the [buttplug-rs-ffi](https://github.com/buttplugio/buttplug-rs-ffi) repo for
more info.

## Hardware Support

Buttplug-rs is currently capable of controlling toys via:

- Bluetooth LE
- Serial Ports
- USB HID
- Lovense Devices via the Lovense Dongle (All Versions)
- XInput gamepads (Windows only at the moment)

See [IOSTIndex](https://iostindex.com) for a full list of supported hardware (Filter on "Buttplug Rust").

## Introduction

[Buttplug](https://buttplug.io) is a framework for hooking up hardware to interfaces, where hardware
usually means sex toys, but could honestly be just about anything. It's basically a userland HID
manager for things that may not specifically be HID.

In more concrete terms, think of Buttplug as something like [osculator](http://www.osculator.net/)
or [VRPN](http://vrpn.org), but for sex toys. Instead of wiimotes and control surfaces, we interface
with vibrators, electrostim equipment, fucking machines, and other hardware that can communicate
with computers.

The core of buttplug works as a router. It is a Rust based application that connects to libraries
that registers and communicates with different hardware. Clients can then connect over websockets or
network ports, to claim and interact with the hardware.

## Compiling

On Windows and macOS, running `cargo build` should suffice for building the project. All
dependencies are vendored in.

On Linux, the following packages will be needed to build with default features:

- `libudev-dev` (Required for serial port/HID support)
- `libusb-1.0-0-dev` (Required for serial port/HID support)

The package names are listed as their debian requirements, and may be different for other
distributions. Removing the `lovense-dongle-manager` and `serial-manager` features should stop these
from being requires.

## Usage

To use Buttplug in your rust application or library, check out the
[buttplug package on crates.io](https://crates.io/buttplug).

The following crate features are available

| Feature | Other Features Used | Description |
| --------- | ----------- | ----------- |
| `client` | None | Buttplug client implementation (in-process connection only) |
| `server` | None | Buttplug server implementation (in-process connection only) |
| `serialize-json` | None | Serde JSON serializer for Buttplug messages, needed for remote connectors |
| `websockets` | `async-std-runtime` | Websocket connectors, used to connect remote clients/servers, with or without SSL |
| `btleplug-manager` | `server` | Bluetooth hardware support on Windows 10, macOS, Linux, iOS |
| `lovense-dongle-manager` | `server` | Lovense USB Dongle support on Windows 7/10, macOS, Linux |
| `serial-manager` | `server` | Serial Port hardware support on Windows 7/10, macOS, Linux |
| `xinput-manager` | `server` | XInput Gamepad support on Windows 7/10 |
| `async-std-runtime` | None | Uses async-std/smol executor for futures |
| `dummy-runtime` | None | Runtime that panics on any spawn. Only used for tests. |
| `thread-pool-runtime` | None | Uses default thread pool executor for futures |

(Tokio coming soon)

Default features are enough to build a full desktop system:

- `thread-pool-runtime`
- `client`
- `server`
- `serialize-json` 
- `websocket`
- `btleplug-manager`
- `serial-manager`
- `lovense-dongle-manager`
- `xinput-manager` (feature is only relevant on windows, but builds as a noop on all
  other platforms).

## Contributing

If you have issues or feature requests, please feel free to [file an
issue](https://github.com/buttplugio/buttplug-rs/issues).

We are not looking for code contributions or pull requests at this time, and will not accept pull
requests that do not have a matching issue where the matter was previously discussed. Pull requests
should only be submitted after talking to [qdot](https://github.com/qdot) via issues (or on
[discord](https://discord.buttplug.io) or [twitter DMs](https://twitter.com/buttplugio) if you would
like to stay anonymous and out of recorded info on the repo) before submitting PRs. Random PRs
without matching issues and discussion are likely to be closed without merging. and receiving
approval to develop code based on an issue. Any random or non-issue pull requests will most likely
be closed without merging.

If you'd like to contribute in a non-technical way, we need money to keep up with supporting the
latest and greatest hardware. We have multiple ways to donate!

- [Patreon](https://patreon.com/qdot)
- [Github Sponsors](https://github.com/sponsors/qdot)
- [Ko-Fi](https://ko-fi.com/qdot76367)

## License

Buttplug is BSD licensed.

    Copyright (c) 2016-2020, Nonpolynomial, LLC
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
