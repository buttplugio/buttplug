use async_std::{
  prelude::StreamExt,
  sync::{channel, Arc, Receiver, Sender},
  task::{self, JoinHandle},
};
use futures::future::Future;
use futures_timer::Delay;
use std::{time::Duration, sync::atomic::{AtomicBool, Ordering}};

pub enum PingMessage {
  Ping,
  StartTimer,
  StopTimer,
}

fn ping_timer(max_ping_time: u64) -> (impl Future<Output = ()>, Sender<PingMessage>, Receiver<()>) {
  let (ping_msg_sender, mut ping_msg_receiver) = channel(256);
  let (pinged_out_sender, pinged_out_receiver) = channel(1);

  let ping_msg_sender_clone = ping_msg_sender.clone();
  let fut = async move {
    let started = false;
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
            handle = Some(task::spawn(async move{
              loop {
                Delay::new(Duration::from_millis(max_ping_time)).await;
                if pinged_clone.load(Ordering::SeqCst) {
                  pinged_clone.store(false, Ordering::SeqCst);
                  continue;
                } else {
                  pinged_out_sender_clone.send(()).await;
                  sender_clone.send(PingMessage::StopTimer).await;
                  break;
                }
              }
            }));
          }
          PingMessage::StopTimer => {
            handle.take();
          }
          PingMessage::Ping => pinged.store(true, Ordering::SeqCst)
        }
      }
    }
  };

  (fut, ping_msg_sender, pinged_out_receiver)
}

pub struct PingTimer {
  ping_msg_sender: Sender<PingMessage>,
  timer_task: JoinHandle<()>,
}

impl PingTimer {
  pub fn new(max_ping_time: u64) -> (Self, Receiver<()>) {
    if max_ping_time == 0 {
      panic!("Can't create ping timer with no max ping time.");
    }
    let (fut, sender, receiver) = ping_timer(max_ping_time);
    (Self {
      timer_task: task::spawn(fut),
      ping_msg_sender: sender
    }, receiver)
  }

  fn send_ping_msg(&self, msg: PingMessage) -> impl Future<Output = ()> {
    let ping_msg_sender = self.ping_msg_sender.clone();
    async move {
      ping_msg_sender.send(msg);
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
