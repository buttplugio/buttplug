use crate::util::async_manager;
use async_channel::{bounded, Receiver, Sender};
use futures::{future::Future, StreamExt};
use futures_timer::Delay;
use std::{
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  time::Duration,
};

pub enum PingMessage {
  Ping,
  StartTimer,
  StopTimer,
  End,
}

fn ping_timer(max_ping_time: u64) -> (impl Future<Output = ()>, Sender<PingMessage>, Receiver<()>) {
  let (ping_msg_sender, mut ping_msg_receiver) = bounded(256);
  let (pinged_out_sender, pinged_out_receiver) = bounded(1);

  let ping_msg_sender_clone = ping_msg_sender.clone();
  let fut = async move {
    let pinged = Arc::new(AtomicBool::new(false));
    let mut handle = None;
    loop {
      while let Some(msg) = ping_msg_receiver.next().await {
        match msg {
          PingMessage::StartTimer => {
            if handle.is_some() {
              continue;
            }
            let sender_clone = ping_msg_sender_clone.clone();
            let pinged_out_sender_clone = pinged_out_sender.clone();
            let pinged_clone = pinged.clone();
            handle = Some(async_manager::spawn(async move {
              loop {
                Delay::new(Duration::from_millis(max_ping_time)).await;
                if pinged_clone.load(Ordering::SeqCst) {
                  pinged_clone.store(false, Ordering::SeqCst);
                  continue;
                } else {
                  error!("Pinged out.");
                  if pinged_out_sender_clone.send(()).await.is_err() {
                    error!("Ping out receiver disappeared, cannot update.");
                  }
                  // This is our own loop, we can unwrap.
                  sender_clone.send(PingMessage::StopTimer).await.unwrap();
                  break;
                }
              }
            }));
          }
          PingMessage::StopTimer => {
            handle.take();
          }
          PingMessage::Ping => pinged.store(true, Ordering::SeqCst),
          PingMessage::End => break,
        }
      }
    }
  };

  (fut, ping_msg_sender, pinged_out_receiver)
}

pub struct PingTimer {
  ping_msg_sender: Sender<PingMessage>,
  // timer_task: JoinHandle<()>,
}

impl Drop for PingTimer {
  fn drop(&mut self) {
    let ping_msg_sender = self.ping_msg_sender.clone();
    async_manager::spawn(async move {
      if ping_msg_sender.send(PingMessage::End).await.is_err() {
        debug!("Receiver does not exist, assuming ping timer event loop already dead.");
      }
    })
    .unwrap();
  }
}

impl PingTimer {
  pub fn new(max_ping_time: u64) -> (Self, Receiver<()>) {
    if max_ping_time == 0 {
      panic!("Can't create ping timer with no max ping time.");
    }
    let (fut, sender, receiver) = ping_timer(max_ping_time);
    async_manager::spawn(fut).unwrap();
    (
      Self {
        // TODO Store this once we can cancel it.
        // timer_task: task::spawn(fut),
        ping_msg_sender: sender,
      },
      receiver,
    )
  }

  fn send_ping_msg(&self, msg: PingMessage) -> impl Future<Output = ()> {
    let ping_msg_sender = self.ping_msg_sender.clone();
    async move {
      if ping_msg_sender.send(msg).await.is_err() {
        error!("Cannot ping, no event loop available.");
      }
    }
  }

  pub fn start_ping_timer(&self) -> impl Future<Output = ()> {
    self.send_ping_msg(PingMessage::StartTimer)
  }

  pub fn stop_ping_timer(&self) -> impl Future<Output = ()> {
    self.send_ping_msg(PingMessage::StopTimer)
  }

  pub fn update_ping_time(&self) -> impl Future<Output = ()> {
    self.send_ping_msg(PingMessage::Ping)
  }
}

// TODO Impl Drop for ping timer that stops the internal async task
