# Buttplug Intiface Device Control Library

[![Patreon donate button](https://img.shields.io/badge/patreon-donate-yellow.svg)](https://www.patreon.com/qdot)
[![Github donate button](https://img.shields.io/badge/github-donate-ff69b4.svg)](https://www.github.com/sponsors/qdot)
[![Discourse Forums](https://img.shields.io/discourse/status?label=buttplug.io%20forums&server=https%3A%2F%2Fdiscuss.buttplug.io)](https://discuss.buttplug.io)
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
    <a href="https://docs.buttplug.io/docs/spec">
      Protocol Spec
    </a>
    <span> | </span>
    <a href="https://docs.buttplug.io/docs">
      Developer Guide
    </a>
    <span> | </span>
    <a href="https://how.do.i.get.buttplug.in">
      User FAQ
    </a>
    <span> | </span>
    <a href="https://awesome.buttplug.io">
      Apps/Games Using Buttplug
    </a>
  </h3>
</div>

<p align="center">
  <img src="https://raw.githubusercontent.com/buttplugio/buttplug/master/images/buttplug_rust_docs.png">
</p>


## Introduction

[Buttplug](https://buttplug.io) is a framework for hooking up hardware to interfaces, where hardware
usually means sex toys, but could honestly be just about anything. It's basically a userland HID
manager for things that may not specifically be HID.

In more concrete terms, think of Buttplug as something like [osculator](http://www.osculator.net/)
or [VRPN](https://vrpn.github.io), but for sex toys. Instead of wiimotes and control surfaces, we interface
with vibrators, electrostim equipment, fucking machines, and other hardware that can communicate
with computers.

The core of buttplug works as a router. It is a Rust based application that connects to libraries
that registers and communicates with different hardware. Clients can then connect over websockets or
network ports, to claim and interact with the hardware.

Buttplug-rs is a full fledged implementation of Buttplug, implementing both the client and server
portions of the system. Implementations for other languages (such as C# and JS) are built on top of
the Rust library. See the [buttplug-rs-ffi](https://github.com/buttplugio/buttplug-rs-ffi) repo for
more info.

## Hardware Support

Buttplug-rs is currently capable of controlling toys via:

- Bluetooth LE (Desktop and Android/iOS)
- Serial Ports (Desktop)
- USB HID (Desktop)
- Lovense Devices via the Lovense Dongle (HID and Serial dongles, Desktop)
- Lovense Connect App (Desktop and Android/iOS)
- Websockets (for simulated and DIY devices, Desktop and Android/iOS)
- XInput gamepads (Windows only)

See [IOSTIndex](https://iostindex.com) for a full list of supported hardware (Filter on "Buttplug Rust").

## Documentation and Examples

To learn how to use the Buttplug Library, check out the [Buttplug Developer Guide](https://docs.buttplug.io/docs). Examples are included in this guide, and for Rust specifically, can be found [in the examples directory of the docs repo](https://github.com/buttplugio/docs.buttplug.io/tree/master/examples/rust).

## Compiling

On Windows and macOS, running `cargo build` should suffice for building the project. All
dependencies are vendored in.

On Linux, the following packages will be needed to build with default features:

- `libudev-dev` (Required for serial port/HID support)
- `libusb-1.0-0-dev` (Required for serial port/HID support)

The package names are listed as their Debian requirements, and may be different for other
distributions. Removing the `lovense-dongle-manager` and `serial-manager` features should stop these
from being required.

## Usage

To use Buttplug in your Rust application or library, check out the
[buttplug package on crates.io](https://crates.io/crates/buttplug).

The following crate features are available

| Feature | Other Features Used | Description |
| --------- | ----------- | ----------- |
| `client` | None | Buttplug client implementation (in-process connection only) |
| `server` | None | Buttplug server implementation (in-process connection only) |
| `serialize-json` | None | Serde JSON serializer for Buttplug messages, needed for remote connectors |
| `websockets` | `tokio-runtime` | Websocket connectors, used to connect remote clients (Clear/SSL)/servers (Clear Only) |
| `btleplug-manager` | `server` | Bluetooth hardware support on Windows >=10, macOS, Linux, iOS, Android |
| `lovense-dongle-manager` | `server` | Lovense USB Dongle support on Windows >=7, macOS, Linux |
| `serial-manager` | `server` | Serial Port hardware support on Windows >=7, macOS, Linux |
| `xinput-manager` | `server` | XInput Gamepad support on Windows >=7 |
| `lovense-connect-service-manager` | `server` | Lovense Connect App support (all platforms) |
| `websocket-server-manager` | `websockets` | Support for connecting devices via Websockets (all platforms) |
| `dummy-runtime` | None | Runtime that panics on any spawn. Only used for tests. |
| `tokio-runtime` | None | Uses tokio for futures |
| `wasm-bindgen-runtime` | None | Uses the wasm-bindgen executor as a runtime (WASM only) |

Default features are enough to build a full desktop system:

- `tokio-runtime`
- `client`
- `server`
- `serialize-json` 
- `websocket`
- `websocket-server-manager`
- `btleplug-manager` (feature builds as noop on WASM)
- `serial-manager` (feature builds as noop on iOS, Android)
- `lovense-dongle-manager` (feature builds as noop on iOS, Android)
- `xinput-manager` (feature is only relevant on windows, but builds as a noop on all
  other platforms).

## Contributing

If you have issues or feature requests, please feel free to [file an
issue](https://github.com/buttplugio/buttplug-rs/issues).

We are not looking for code contributions or pull requests at this time, and will not accept pull
requests that do not have a matching issue where the matter was previously discussed. Pull requests
should only be submitted after talking to [qdot](https://github.com/qdot) via issues (or on
[Discord](https://discord.buttplug.io) or [Twitter DMs](https://twitter.com/buttplugio) if you would
like to stay anonymous and out of recorded info on the repo) and receiving approval to develop code
based on an issue. Any random or non-issue pull requests will most likely be closed without merging.

If you'd like to contribute in a non-technical way, we need money to keep up with supporting the
latest and greatest hardware. We have multiple ways to donate!

- [Patreon](https://patreon.com/qdot)
- [Github Sponsors](https://github.com/sponsors/qdot)
- [Ko-Fi](https://ko-fi.com/qdot76367)

## License

Buttplug is BSD 3-Clause licensed.

```text

Copyright (c) 2016-2022, Nonpolynomial, LLC
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
