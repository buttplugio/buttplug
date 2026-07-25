// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use anyhow::Result;
use vergen_gitcl::{Build, Emitter, Gitcl};

fn main() -> Result<()> {
  let build = Build::builder().build_timestamp(true).build();
  let gitcl = Gitcl::builder().sha(true).build();

  Emitter::default()
    .add_instructions(&build)?
    .add_instructions(&gitcl)?
    .emit()?;

  Ok(())
}
