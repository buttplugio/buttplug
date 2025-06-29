// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use async_stream::stream;
use futures::{pin_mut, Stream};
use tokio::sync::broadcast;

pub fn convert_broadcast_receiver_to_stream<T>(
  receiver: broadcast::Receiver<T>,
) -> impl Stream<Item = T>
where
  T: Unpin + Clone,
{
  stream! {
    pin_mut!(receiver);
    while let Ok(val) = receiver.recv().await {
      yield val;
    }
  }
}
