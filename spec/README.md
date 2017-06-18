# Buttplug Protocol and Architecture Documentation

[![Build Status](https://img.shields.io/travis/metafetish/buttplug.svg)](https://travis-ci.org/metafetish/lovesense-js)

This repo houses architecture documentation and the central json
schema for the Buttplug Sex Toy Control Server Protocol.

More information on Buttplug and the hardware it supports can be found
at [https://buttplug.io](https://buttplug.io).

## Buttplug Message Format JSON Schema

The JSON Schema for the Buttplug Message format is available in the
'schema' directory. This is the central source of truth for
communicating with Buttplug clients and servers. The goal is to only
update and not remove messages, but as with all protocols, this goal
may be more of a dream than a reality. We'll be abiding by semantic
versioning rules, with major version changes only on breaking changes
to the schema.

## Buttplug Server Implementations

If you are looking for a server implementation of the Buttplug
Protocol, here's a list of the ones we are aware of.

- [buttplug-csharp](http://github.com/metafetish/buttplug-csharp):
  C#/.Net implementation of the Buttplug Server.
  - Status: Mostly done!
- [buttplug-rs](http://github.com/metafetish/buttplug-rs): Rust
  implementation of the Buttplug Server.
  - Status: Half started, needs to be brought up to date with the new
    JSON schema.
- [buttplug-js](http://github.com/metafetish/buttplug-js): Javascript
  implementation of the Buttplug Protocol (webclient and node.js), and
  Node.js implementation of the Buttplug Server.
  - Status: Barely started.
