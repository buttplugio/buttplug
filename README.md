# Buttplug

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


## License

Buttplug is BSD licensed.

    Copyright (c) 2019, Nonpolynomial Labs, LLC
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
