# Buttplug Protocol and Architecture Documentation

[![Netlify Status](https://api.netlify.com/api/v1/badges/ca7221a2-36a6-4362-8459-07a4428c60b4/deploy-status)](https://app.netlify.com/sites/buttplug-spec/deploys)

[![Patreon donate button](https://img.shields.io/badge/patreon-donate-yellow.svg)](https://www.patreon.com/qdot)
[![Discourse Forum](https://img.shields.io/discourse/https/metafetish.club/topics.svg)](https://metafetish.club)
[![Discord](https://img.shields.io/discord/353303527587708932.svg?logo=discord)](https://discord.gg/t9g9RuD)
[![Twitter](https://img.shields.io/twitter/follow/buttplugio.svg?style=social&logo=twitter)](https://twitter.com/buttplugio)

## Table of Contents

* [Introduction](#introduction)
* [Talk To Us!](#talk-to-us)
* [Building The Protocol Documents](#building-the-protocol-documents)
* [Buttplug Documentation and Tutorials](#buttplug-documentation-and-tutorials)
* [Buttplug Repos and Supporting Applications](#buttplug-implementation-repos-and-supporting-applications)
* [Support The Project](#support-the-project)
* [License](#license)

## Introduction

This repo houses specifications documents for the Buttplug Intimate
Hardware Control Server Protocol (referred to hereafter as Buttplug).

Buttplug aims to simplify accessing and controlling different kinds of
intimate hardware such as vibrators, strokers, and machines, as well
as other devices like kegelcizers, electrostimulation systems, and
whatever else seems like it could be intimate. The goal is to abstract
line level (usb/bluetooth/serial/etc) protocol knowledge away from the
developer, so they can concentrate on creating new and interesting
interfaces, applications, games, and other software.

HTML rendered version of the Protocol Spec is at
[https://buttplug-spec.docs.buttplug.io/](https://buttplug-spec.docs.buttplug.io/).

More information on the Buttplug project can be found at at
[https://buttplug.io](https://buttplug.io).

## Talk To Us!

If you're interested in Buttplug Development, check out our [discord server](https://discord.buttplug.io)!

There's also a message board for development and support at [metafetish.club](https://metafetish.club).

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

## Buttplug Documentation and Tutorials

### Protocol, Schema, and System Documentation

- [Buttplug Protocol Spec](http://github.com/buttplugio/buttplug): Repo containing
  the specification document for the Buttplug sex toy control
  protocol.
- [Buttplug Protocol JSON Schema](http://github.com/buttplugio/buttplug-schema):
  JSON Schema for the Buttplug Protocol Standard. Usually subtree'd
  into server/client implementations.
- [Buttplug Developer Guide](http://github.com/buttplugio/buttplug-developer-guide):
  Manual for developing applications that use Buttplug, as well as
  information on Buttplug Client and Server architecture.
- [STPIHKAL](http://github.com/buttplugio/stpihkal): "Sex Toy
  Protocols I Have Known And Loved", a book containing low-level
  proprietary protocol specifications for sex toys and sex hardware,
  as well as movie synchronization formats and other miscellaneous
  information.

### Tutorials

- [how-to-buttplug Glitch.come
  project](https://glitch.com/edit/#!/how-to-buttplug?path=README.md) -
  Tutorial project for using buttplug-js for web applications.

## Buttplug Implementation Repos and Supporting Applications

### Server Implementations

- [buttplug-csharp](http://github.com/buttplugio/buttplug-csharp):
  C#/.Net implementation of the Buttplug Server for Win7/10
  - Status: Stable
  - Packages: [Available on nuget](https://www.nuget.org/packages?q=buttplug)
  - Maintainers: Core Buttplug Team
- [buttplug-js](http://github.com/buttplugio/buttplug-js): Javascript/Typescript 
  implementation of the Buttplug Server for Web (using WebBluetooth) and Node.js
  - Status: Stable
  - Packages: [Available on npm](https://www.npmjs.com/package/buttplug)
  - Maintainers: Core Buttplug Team
- [buttplug-rs](http://github.com/buttplugio/buttplug-rs): Rust
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
- [buttplug-twine](https://github.com/buttplugio/buttplug-twine):
  Twine v2 macros for using buttplug-js with Twine stories.

## Support The Project

If you find this project helpful, you can [support us via
Patreon](http://patreon.com/qdot)! Every donation helps us afford more
hardware to reverse, document, and write code for!

## License

Buttplug is BSD licensed.

    Copyright (c) 2016-2018, Nonpolynomial Labs, LLC
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
