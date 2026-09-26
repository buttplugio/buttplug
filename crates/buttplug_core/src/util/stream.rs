// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use async_stream::stream;
use futures::{Stream, pin_mut};
use log::warn;
use tokio::sync::broadcast::{self, error::RecvError};

/// Lagged messages are dropped with a warning; only channel close ends the stream.
pub fn convert_broadcast_receiver_to_stream<T>(
  receiver: broadcast::Receiver<T>,
) -> impl Stream<Item = T>
where
  T: Unpin + Clone,
{
  stream! {
    pin_mut!(receiver);
    loop {
      match receiver.recv().await {
        Ok(val) => yield val,
        Err(RecvError::Lagged(n)) => {
          warn!(
            "Broadcast receiver for {} lagged, dropped {} messages",
            std::any::type_name::<T>(),
            n
          );
        }
        Err(RecvError::Closed) => break,
      }
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use futures::StreamExt;

  #[tokio::test]
  async fn test_stream_continues_after_lag() {
    let (sender, receiver) = broadcast::channel(2);
    let stream = convert_broadcast_receiver_to_stream(receiver);
    pin_mut!(stream);

    for i in 0..5 {
      sender.send(i).unwrap();
    }

    assert_eq!(stream.next().await, Some(3));
    assert_eq!(stream.next().await, Some(4));

    sender.send(5).unwrap();
    assert_eq!(stream.next().await, Some(5));

    drop(sender);
    assert_eq!(stream.next().await, None);
  }

  #[tokio::test]
  async fn test_stream_ends_when_sender_dropped() {
    let (sender, receiver) = broadcast::channel::<i32>(2);
    let stream = convert_broadcast_receiver_to_stream(receiver);
    pin_mut!(stream);

    sender.send(1).unwrap();
    sender.send(2).unwrap();
    drop(sender);

    assert_eq!(stream.next().await, Some(1));
    assert_eq!(stream.next().await, Some(2));
    assert_eq!(stream.next().await, None);
  }
}
