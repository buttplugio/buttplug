// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

fn main() {
  prost_build::compile_protos(
    &[
      "src/server/device/protocol/thehandy/protocomm.proto",
      "src/server/device/protocol/thehandy/handyplug.proto",
    ],
    &["src/server/device/protocol/thehandy"],
  )
  .expect("These will always compile or we shouldn't be building.");
}
