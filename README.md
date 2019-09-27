# Buttplug

[![Patreon donate button](https://img.shields.io/badge/patreon-donate-yellow.svg)](https://www.patreon.com/qdot)
[![Discourse Forum](https://img.shields.io/badge/discourse-forum-blue.svg)](https://metafetish.club)
[![Discord](https://img.shields.io/discord/353303527587708932.svg?logo=discord)](https://discord.buttplug.io)
[![Twitter](https://img.shields.io/twitter/follow/buttplugio.svg?style=social&logo=twitter)](https://twitter.com/buttplugio)

Rust implementation of the Buttplug Intimate Hardware Protocol,
including implementations of the client and server.

## Read Me First!

If you are new to Buttplug, you most likely want to start with the
[Buttplug Website](https://buttplug.io) or the [Buttplug Core
Repo](https://github.com/buttplugio/buttplug).

For a demo of what this framework can do, [check out this demo
video](https://www.youtube.com/watch?v=RXD76g5fias).

The Rust code in this repo is currently being rebuilt and rewritten.
Our [C#](https://github.com/metafetish/buttplug-csharp) and
[Typescript/JS/Node](https://github.com/metafetish/buttplug-js)
implementations are the most complete for the moment.

## Introduction

[Buttplug](https://buttplug.io) is a framework for hooking up hardware
to interfaces, where hardware usually means sex toys, but could
honestly be just about anything. It's basically a userland HID manager
for things that may not specifically be HID.

In more concrete terms, think of Buttplug as something like
[osculator](http://www.osculator.net/) or [VRPN](http://vrpn.org), but
for sex toys. Instead of wiimotes and control surfaces, we interface
with vibrators, electrostim equipment, fucking machines, and other
hardware that can communicate with computers.

The core of buttplug works as a router. It is a Rust based application
that connects to libraries that registers and communicates with
different hardware. Clients can then connect over websockets or
network ports, to claim and interact with the hardware.

## Architecture

**Warning:** Here lies monsters. Or at least, stupidity. Everything I
am about to say may end up fabulously wrong, in which case, enjoy the
comedy of errors that is this repo.

This section is a discussion of the idea behind the new (Sept 2019)
Rust implementation of Buttplug.

Understanding the following discussion is probably only possible if
you check out the [Buttplug Protocol
Spec](https://buttplug-spec.docs.buttplug.io) first. This also assumes
some knowledge of Buttplug reference library architecture. If you're
really curious and have questions, [join our discord and ask
qDot](https://discord.buttplug.io).

The Rust implementation of Buttplug is meant to be a canonical,
reusable core library, accessible from other programming languages via
FFI boundaries.

The core library consists of:

- Core
  - Protocol Message definitions, mostly.
- Devices
  - Proprietary device protocols. These take in Buttplug Control
    Commands, output ButtplugRawCmds. Similarly, they can also
    translate and emit input messages from hardware.
  - Device configuration file handling. This is the external file we
    use for defining device identifiers. This includes USB
    Vendor/Product IDs, Bluetooth LE Names and Service/Characteristic
    UUIDs, etc.
- Server
  - Coordinates talking to hardware, via DeviceSubtypeManager objects.
    DeviceSubtypeManagers handle platform specific communication bus
    access, i.e. USB, Bluetooth, Serial, etc. **NOTE**:
    DeviceSubtypeManagers may exist across an FFI boundry. They are
    not required to be implemented in Rust.
- Client
  - API used by applications to communicate with a server. Abstracts
    communication via connectors, so that a Server can exist
    in-process, or out-of-process via some sort of IPC (pipes,
    websockets, carrier pidgeon, etc...). Also exposes FFI bindings
    for client access in languages other than Rust.
    
Things not in the core library but still needed by Buttplug include
either platform/OS specific or optional components, such as:

- DeviceSubtypeManagers
  - Talking to Bluetooth/USB/Serial/Etc usually takes platform
    specific calls, which sometimes may be easier to make from other
    languages. For instance, calling UWP Bluetooth on Windows is
    easier from C#. Therefore, we maintain an FFI boundary with a
    protobuf protocol here.
- Client Connectors
  - Clients may need to talk to Servers that exist in other processes.
    Therefore we provide Connectors, which allow different types of
    IPC. These feed clients with messages from the IPC, and are
    usually external implementations as not all uses of Buttplug will
    require all types of IPC.
    
In prior implementations of Buttplug, including C# and Typescript, the
core usually existed expecting implementations of
DeviceSubtypeManagers and Connectors in the same language. As both of
these languages require specific runtimes (.Net for C#, node or a
browser for Typescript/Javascript), making them cross platform
required rewriting the core implementation. By using Rust, the hope is
to keep one reference implementation of the core library, and wrapping
that in FFIs for other implementations or compiling to WASM or other
intermediaries as needed. 

This is hopefully easier than, say, hauling the whole .Net platform
along with us everywhere we go.

To support other languages, the plan is to have multiple FFI
boundaries and possible IPC tunnels.

- The client API will be accessible via FFI, so we can build language
  bindings on top of it.
- The layer between the Client and Server has always existed as an IPC
  boundary outside of cases where the client/server are in the same
  process.
- The server will have an FFI boundary on devices, so we can implement
  platform specific device handling in other languages if needed
  (Swift, Java, C#, etc.)
  
Incoming messages will still be JSON, because we made a bad decision
early on and by god we're sticking with it. Internal boundaries (like
the server/DeviceSubtypeManager boundary) will either be callback
based, or message-passing via protobuf protocols.

## License

Buttplug is BSD licensed.

    Copyright (c) 2016-2019, Nonpolynomial Labs, LLC
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
