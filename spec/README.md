# Buttplug Protocol and Architecture Documentation

[![Patreon donate button](https://img.shields.io/badge/patreon-donate-yellow.svg)](https://www.patreon.com/qdot)

## Table of Contents

* [Introduction](#introduction)
* [Building The Protocol Documents](#building-the-protocol-documents)
* [Buttplug Repos and Supporting Applications](#buttplug-repos-and-supporting-applications)
* [Support The Project](#support-the-project)
* [License](#license)

## Introduction

This repo houses specifications documents for the Buttplug Sex Toy
Control Server Protocol.

HTML rendered version of the Protocol Spec is at [https://metafetish.github.io/buttplug](https://metafetish.github.io/buttplug).

More information on Buttplug and the hardware it supports can be found
at [https://buttplug.io](https://buttplug.io).

## Building The Protocol Documents

To build this document into HTML pages or pdfs, you'll need to use
gitbook. We have a node package file available to install the version
we use for development and build the book. Use the following commands
to set it up.

```
npm install
npm run build
```

This will build the HTML version of the book into the _book directory.

We keep a built version of the HTML version in our gh-pages branch,
and it is updated on every commit to master.

## Buttplug Repos and Supporting Applications

### Protocol, Schema, and System Documentation

- [Buttplug Protocol Spec](http://github.com/metafetish/buttplug): Repo containing
  the specification document for the Buttplug sex toy control
  protocol.
- [Buttplug Protocol JSON Schema](http://github.com/metafetish/buttplug-schema):
  JSON Schema for the Buttplug Protocol Standard. Usually subtree'd
  into server/client implementations.
- [Buttplug Developer Guide](http://github.com/metafetish/buttplug-developer-guide):
  Manual for developing applications that use Buttplug, as well as
  information on Buttplug Client and Server architecture.
- [STPIHKAL](http://github.com/metafetish/stpihkal): "Sex Toy
  Protocols I Have Known And Loved", a book containing low-level
  proprietary protocol specifications for sex toys and sex hardware,
  as well as movie synchronization formats and other miscellaneous
  information.

### Server Implementations

If you are looking for a server implementation of the Buttplug
Protocol, here's a list of the ones we are aware of.

- [buttplug-csharp](http://github.com/metafetish/buttplug-csharp):
  C#/.Net implementation of the Buttplug Server for Win7/10
  - Status: Stable
  - Packages: [Available on nuget](https://www.nuget.org/packages?q=buttplug)
  - Maintainers: Core Buttplug Team
- [buttplug-js](http://github.com/metafetish/buttplug-js): Javascript/Typescript 
  implementation of the Buttplug Server for Web (using WebBluetooth) and Node.js
  - Status: Stable
  - Packages: [Available on npm](https://www.npmjs.com/package/buttplug)
  - Maintainers: Core Buttplug Team
- [buttplug-rs](http://github.com/metafetish/buttplug-rs): Rust
  implementation of the Buttplug Server.
  - Status: Under development, needs to be brought up to date with the new
    JSON schema.
  - Maintainers: Core Buttplug Team

### Client Implementations

- [buttplug-csharp](http://github.com/metafetish/buttplug-csharp):
  C#/.Net implementation of the Buttplug Client for Win7/10
  - Status: Stable
  - Packages: [Available on nuget](https://www.nuget.org/packages?q=buttplug)
  - Maintainers: Core Buttplug Team 
- [buttplug-js](http://github.com/metafetish/buttplug-js): Javascript
  implementation of the Buttplug Protocol Client
  - Status: Stable
  - Packages: [Available on npm](https://www.npmjs.com/package/buttplug)
  - Maintainers: Core Buttplug Team 
- [golibbuttplug](https://github.com/funjack/golibbuttplug): Go
  implementation of the Buttplug Protocol Client
  - Status: Stable
  - Maintainers: Community Maintained

### Supporting Applications

- [SyncyDink](http://github.com/metafetish/syncydink):
  Javascript/Typescript Haptic Video player for the web.
- [Playground](http://github.com/metafetish/buttplug-playground):
  Javascript/Typescript device testing web application.
- [ScriptPlayer](http://github.com/FredTungsten/ScriptPlayer): C#
  Haptic Video player.
- [LaunchControl](http://github.com/funjack/launchcontrol): Go Haptic
  Video Player and Launch Control Scripts.
- [buttplug-twine](https://github.com/metafetish/buttplug-twine):
  Twine v2 macros for using buttplug-js with Twine stories.

## Support The Project

If you find this project helpful, you
can
[support Metafetish projects via Patreon](http://patreon.com/qdot)!
Every donation helps us afford more hardware to reverse, document, and
write code for!

## License

Buttplug is BSD licensed.

    Copyright (c) 2016, Metafetish
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
