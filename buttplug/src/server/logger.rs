use crate::{
  core::messages::{self, ButtplugServerMessage},
  util::async_manager,
};
use async_channel::Sender;
use tracing::{Event, subscriber::Subscriber};
use tracing_subscriber::{layer::{Layer, Context}};
use std::fmt::{self, Write};
use tracing::field::{Visit, Field};

pub struct ButtplugLogHandler {
  level: messages::LogLevel,
  message_sender: Sender<ButtplugServerMessage>,
}

pub struct StringVisitor<'a> {
    string: &'a mut String,
}

impl<'a> Visit for StringVisitor<'a> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
      if field.name() == "message" {
        write!(self.string, "{:?}", value).unwrap();
      } else {
        write!(self.string, "{} = {:?}; ", field.name(), value).unwrap();
      }
    }
}

impl<S: Subscriber> Layer<S> for ButtplugLogHandler {
  fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
    if messages::LogLevel::from(event.metadata().level().clone()) > self.level ||
     event.metadata().fields().field("message").is_none() {
      return;
    }
    let sender_clone = self.message_sender.clone();
    let level: messages::LogLevel = messages::LogLevel::from(event.metadata().level().clone());
    let mut log_message = String::new();

    event.record(&mut StringVisitor { string: &mut log_message });

    let log_msg = format!("[{}] {}", event.metadata().target(), log_message);
    async_manager::spawn(async move {
      // TODO If our sender fails, it'd be nice to be able to kill our
      // subscriber, but I'm not sure how? We can't log here, as it'd cause a
      // recursive log call.
      let _ = sender_clone
        .send(messages::Log::new(level, &log_msg).into())
        .await;
    }).unwrap();
  }
}

impl ButtplugLogHandler {
  pub fn new(level: &messages::LogLevel, message_sender: Sender<ButtplugServerMessage>) -> Self {
    Self {
      level: level.clone().into(),
      message_sender,
    }
  }

  pub fn set_level(&mut self, level: messages::LogLevel) {
    self.level = level;
  }
}

#[cfg(test)]
mod test {
  use super::ButtplugLogHandler;
  use crate::core::messages;
  use tracing_subscriber::layer::SubscriberExt;
  use futures::StreamExt;
  use async_channel;
  use async_std::task;

  #[test]
  fn test_layer_subscription() {
    let (sender, mut receiver) = async_channel::bounded(256);
    let fmtsub = tracing_subscriber::fmt()
      .finish()
      .with(ButtplugLogHandler::new(&messages::LogLevel::Debug, sender));
    tracing::subscriber::set_global_default(fmtsub).unwrap();
    info!("Test message");
    task::block_on(async move {
      match receiver.next().await {
        Some(msg) => {
          if let messages::ButtplugServerMessage::Log(log_msg) = msg {
            assert!(log_msg.log_message.contains("Test message"));
          } else {
            panic!("Got wrong message type: {:?}", msg);
          }
        }
        None => panic!("Should't drop sender"),
      };
    });
  }
}