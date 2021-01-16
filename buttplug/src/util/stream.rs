use async_stream::stream;
use tokio::sync::{broadcast, mpsc};
use futures::{Stream, FutureExt};

pub fn convert_broadcast_receiver_to_stream<T>(receiver: broadcast::Receiver<T>) -> impl Stream<Item = T>
where T: Unpin + Clone {
  stream! {
    pin_mut!(receiver);
    while let Ok(val) = receiver.recv().await {
      yield val;
    }
  }
}

pub fn recv_now<T>(receiver: &mut mpsc::Receiver<T>) -> Option<Option<T>> {
  receiver.recv().now_or_never()
}

pub fn iffy_is_empty_check<T>(receiver: &mut mpsc::Receiver<T>) -> bool {
  recv_now(receiver).is_none()
}