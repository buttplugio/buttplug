// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::util::{async_manager, sleep};
use futures::{Future, FutureExt};
use std::{
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  time::Duration,
};
use tokio::sync::{mpsc, Notify};

pub enum PingMessage {
  Ping,
  StartTimer,
  StopTimer,
  End,
}

async fn ping_timer(
  max_ping_time: u32,
  mut ping_msg_receiver: mpsc::Receiver<PingMessage>,
  notifier: Arc<Notify>,
  pinged_out_status: Arc<AtomicBool>,
) {
  let mut started = false;
  let mut pinged = false;
  loop {
    select! {
      _ = sleep(Duration::from_millis(max_ping_time.into())).fuse() => {
        if started {
          if !pinged {
            notifier.notify_waiters();
            pinged_out_status.store(true, Ordering::SeqCst);
            return;
          }
          pinged = false;
        }
      }
      msg = ping_msg_receiver.recv().fuse() => {
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
  ping_timeout_notifier: Arc<Notify>,
  pinged_out: Arc<AtomicBool>,
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
  pub fn new(max_ping_time: u32) -> Self {
    let ping_timeout_notifier = Arc::new(Notify::new());
    let (sender, receiver) = mpsc::channel(256);
    let pinged_out = Arc::new(AtomicBool::new(false));
    if max_ping_time > 0 {
      let fut = ping_timer(
        max_ping_time,
        receiver,
        ping_timeout_notifier.clone(),
        pinged_out.clone(),
      );
      async_manager::spawn(async move { fut.await });
    }
    Self {
      max_ping_time,
      ping_msg_sender: sender,
      ping_timeout_notifier,
      pinged_out,
    }
  }

  pub fn ping_timeout_waiter(&self) -> impl Future<Output = ()> {
    let notify = self.ping_timeout_notifier.clone();
    async move {
      notify.notified().await;
    }
  }

  fn send_ping_msg(&self, msg: PingMessage) -> impl Future<Output = ()> {
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

  pub fn start_ping_timer(&self) -> impl Future<Output = ()> {
    // If we're starting the timer, clear our status.
    self.pinged_out.store(false, Ordering::SeqCst);
    self.send_ping_msg(PingMessage::StartTimer)
  }

  pub fn stop_ping_timer(&self) -> impl Future<Output = ()> {
    self.send_ping_msg(PingMessage::StopTimer)
  }

  pub fn update_ping_time(&self) -> impl Future<Output = ()> {
    self.send_ping_msg(PingMessage::Ping)
  }

  pub fn pinged_out(&self) -> bool {
    self.pinged_out.load(Ordering::SeqCst)
  }
}
