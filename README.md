# Buttplug

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

Main repo for the Buttplug project, including the protocol spec, device config, and a full Rust implementation of the Buttplug Intimate Hardware Control Protocol.

This repo is a monorepo with multiple projects, including:

- [Buttplug Protocol Specification](spec/) - Protocol spec and docs
- [buttplug-schema](buttplug/buttplug-schema) - JSON schema for the Buttplug protocol spec
- [buttplug-device-config](buttplug/buttplug-device-config) - Device configuration file for buttplug (where we store all of the device identifiers)
- [buttplug](buttplug/) - Rust implementation of the Buttplug protocol spec
- [buttplug_device](buttplug_derive/) - Procedural macros used by the buttplug rust library.


For information about compiling and using these libraries, please check the
README files in their directories.

For a list of applications using Buttplug, see the [awesome-buttplug repo](https://github.com/buttplugio/awesome-buttplug).

## Other Language Implementations

See the [awesome-buttplug repo](https://github.com/buttplugio/awesome-buttplug) for a full list of implementations.

- [Buttplug C#](https://github.com/buttplugio/buttplug-rs-ffi/tree/master/csharp)
- [Buttplug JS/Typescript/WASM](https://github.com/buttplugio/buttplug-rs-ffi/tree/master/js)
- [Buttplug Python](https://github.com/buttplugio/buttplug-py)