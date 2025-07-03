use anyhow::Result;
use vergen_gitcl::{BuildBuilder, Emitter, GitclBuilder};

fn main() -> Result<()> {
  let build = BuildBuilder::default().build_timestamp(true).build()?;
  let gitcl = GitclBuilder::default().sha(true).build()?;

  Emitter::default()
    .add_instructions(&build)?
    .add_instructions(&gitcl)?
    .emit()?;

  Ok(())
}
