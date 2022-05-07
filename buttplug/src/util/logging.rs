// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::util::async_manager;
use tokio::sync::mpsc::Sender;

use tracing_subscriber::fmt::MakeWriter;

/// Convenience struct for handling tracing output from Buttplug.
///
/// Since Buttplug uses tracing for logging internally, we expect executables to
/// handle setting up the outputs. However, there are a few situations we deal
/// with where we want to shove out to a channel instead of stdout or other
/// writers. We just shove out a Vec<u8> and expect the other end to do whatever
/// string parsing it might need.
pub struct ChannelWriter {
  log_sender: Sender<Vec<u8>>,
}

impl ChannelWriter {
  pub fn new(sender: Sender<Vec<u8>>) -> Self {
    Self { log_sender: sender }
  }
}

impl std::io::Write for ChannelWriter {
  fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
    let sender = self.log_sender.clone();
    let len = buf.len();
    let send_buf = buf.to_vec();
    async_manager::spawn(async move {
      // Ignore errors on dropped channels here. We can't really do a ton about
      // them.
      let _ = sender.send(send_buf.to_vec()).await;
    });
    Ok(len)
  }

  fn flush(&mut self) -> Result<(), std::io::Error> {
    Ok(())
  }
}

impl MakeWriter<'_> for ChannelWriter {
  type Writer = ChannelWriter;
  fn make_writer(&self) -> Self::Writer {
    ChannelWriter::new(self.log_sender.clone())
  }
}
