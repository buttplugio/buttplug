// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_core::util::{async_manager, sleep};
use futures::Future;
use std::{sync::Arc, time::Duration};
use tokio::{
  select,
  sync::{Mutex, mpsc},
};

pub enum PingMessage {
  Ping,
  StartTimer,
  StopTimer,
  End,
}

/// Internal ping timer task that monitors for ping timeouts.
/// When a timeout occurs, it calls the provided callback.
async fn ping_timer<F>(
  max_ping_time: u32,
  mut ping_msg_receiver: mpsc::Receiver<PingMessage>,
  on_ping_timeout: Arc<Mutex<Option<F>>>,
) where
  F: FnOnce() + Send + 'static,
{
  let mut started = false;
  let mut pinged = false;
  loop {
    select! {
      _ = sleep(Duration::from_millis(max_ping_time.into())) => {
        if started {
          if !pinged {
            // Ping timeout occurred - call the callback
            if let Some(callback) = on_ping_timeout.lock().await.take() {
              callback();
            }
            return;
          }
          pinged = false;
        }
      }
      msg = ping_msg_receiver.recv() => {
        if msg.is_none() {
          return;
        }
        match msg.expect("Already checked") {
          PingMessage::StartTimer => started = true,
          PingMessage::StopTimer => started = false,
          PingMessage::Ping => pinged = true,
          PingMessage::End => break,
        }
      }
    };
  }
}

pub struct PingTimer {
  max_ping_time: u32,
  ping_msg_sender: mpsc::Sender<PingMessage>,
}

impl Drop for PingTimer {
  fn drop(&mut self) {
    // This cannot block, otherwise it will throw in WASM contexts on
    // destruction. We must use send(), not blocking_send().
    let sender = self.ping_msg_sender.clone();
    async_manager::spawn(async move {
      if sender.send(PingMessage::End).await.is_err() {
        debug!("Receiver does not exist, assuming ping timer event loop already dead.");
      }
    });
  }
}

impl PingTimer {
  /// Create a new PingTimer with an optional callback for ping timeout.
  ///
  /// The callback is called once when the ping timer expires without receiving
  /// a ping message. If max_ping_time is 0, the timer is disabled and the
  /// callback will never be called.
  pub fn new<F>(max_ping_time: u32, on_ping_timeout: Option<F>) -> Self
  where
    F: FnOnce() + Send + 'static,
  {
    let (sender, receiver) = mpsc::channel(256);
    if max_ping_time > 0 {
      let callback = Arc::new(Mutex::new(on_ping_timeout));
      let fut = ping_timer(max_ping_time, receiver, callback);
      async_manager::spawn(fut);
    }
    Self {
      max_ping_time,
      ping_msg_sender: sender,
    }
  }

  fn send_ping_msg(&self, msg: PingMessage) -> impl Future<Output = ()> + use<> {
    let ping_msg_sender = self.ping_msg_sender.clone();
    let max_ping_time = self.max_ping_time;
    async move {
      if max_ping_time == 0 {
        return;
      }
      if ping_msg_sender.send(msg).await.is_err() {
        error!("Cannot ping, no event loop available.");
      }
    }
  }

  pub fn start_ping_timer(&self) -> impl Future<Output = ()> + use<> {
    self.send_ping_msg(PingMessage::StartTimer)
  }

  pub fn stop_ping_timer(&self) -> impl Future<Output = ()> + use<> {
    self.send_ping_msg(PingMessage::StopTimer)
  }

  pub fn update_ping_time(&self) -> impl Future<Output = ()> + use<> {
    self.send_ping_msg(PingMessage::Ping)
  }
}
