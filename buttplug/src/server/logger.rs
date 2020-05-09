use crate::core::messages::{self, ButtplugOutMessage};
use async_std::{sync::Sender, task};
use log::{Level, Log, Metadata, Record};

pub struct ButtplugLogHandler {
  level: Level,
  message_sender: Sender<ButtplugOutMessage>,
}

impl ButtplugLogHandler {
  pub fn new(level: &messages::LogLevel, message_sender: Sender<ButtplugOutMessage>) -> Self {
    Self {
      level: level.clone().into(),
      message_sender,
    }
  }
}

impl Log for ButtplugLogHandler {
  fn enabled(&self, metadata: &Metadata) -> bool {
    metadata.level() <= self.level
  }

  fn log(&self, record: &Record) {
    if self.enabled(record.metadata()) {
      let target = if record.target().len() > 0 {
        record.target()
      } else {
        record.module_path().unwrap_or_default()
      };

      let sender_clone = self.message_sender.clone();
      let level: messages::LogLevel = record.level().into();
      let log_msg = format!("[{}] {}", target, record.args());
      task::spawn(async move {
        sender_clone
          .send(messages::Log::new(level, log_msg).into())
          .await;
      });
    }
  }

  fn flush(&self) {
  }
}
