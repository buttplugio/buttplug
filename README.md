# Buttplug

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

A Rust implementation of the Buttplug Intimate Hardware Control Protocol, including a client and server. This is the core implementation of Buttplug.

This repo is a monorepo with multiple projects, including:

- [buttplug](buttplug/) - Rust implementation of the Buttplug protocol spec
- [buttplug-schema](buttplug/buttplug-schema) - JSON schema for the Buttplug protocol spec
- [buttplug-device-config](buttplug/buttplug-device-config) - Device configuration file for buttplug
  (where we store all of the device identifiers)
- [buttplug_derive](buttplug_derive/) - Procedural macros used by the buttplug rust library.

For information about compiling and using these libraries, please check the
README files in their directories.

For a list of applications using Buttplug, see the [awesome-buttplug repo](https://github.com/buttplugio/awesome-buttplug).

## Other Language Implementations

See the [awesome-buttplug repo](https://github.com/buttplugio/awesome-buttplug#development-and-libraries) for a full list of implementations.
