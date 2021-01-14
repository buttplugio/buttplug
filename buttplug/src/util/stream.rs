use async_stream::stream;
use tokio::sync::broadcast;
use futures::Stream;

pub fn convert_broadcast_receiver_to_stream<T>(mut receiver: broadcast::Receiver<T>) -> impl Stream<Item = T>
where T: Unpin + Clone {
  stream! {
    pin_mut!(receiver);
    while let Ok(val) = receiver.recv().await {
      yield val;
    }
  }
}