use crate::core::messages::{ButtplugMessage, ButtplugMessageUnion};
use core::pin::Pin;
use futures::prelude::Future;
use futures::task::{Context, Poll, Waker};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Method copypasta'd from https://rust-lang.github.io/async-book/02_executor/03_wakeups.html

#[derive(Default, Debug)]
pub struct ClientConnectorMessageState {
    pub reply_msg: Option<ButtplugMessageUnion>,
    pub waker: Option<Waker>,
}

type ClientConnectorMessageStateShared = Arc<Mutex<ClientConnectorMessageState>>;

#[derive(Default, Debug)]
pub struct ClientConnectorMessageFuture {
    waker_state: ClientConnectorMessageStateShared,
}

impl ClientConnectorMessageFuture {
    fn new(state: &ClientConnectorMessageStateShared) -> ClientConnectorMessageFuture {
        ClientConnectorMessageFuture {
            waker_state: state.clone(),
        }
    }
}

impl Future for ClientConnectorMessageFuture {
    type Output = ButtplugMessageUnion;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let mut waker_state = self.waker_state.lock().unwrap();
        if waker_state.reply_msg.is_some() {
            let msg = waker_state.reply_msg.take().unwrap();
            Poll::Ready(msg)
        } else {
            println!("Got waker!");
            waker_state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

pub struct ClientConnectorMessageSorter {
    future_map: HashMap<u32, ClientConnectorMessageStateShared>,
    current_id: u32,
}

impl ClientConnectorMessageSorter {
    pub fn new() -> ClientConnectorMessageSorter {
        ClientConnectorMessageSorter {
            future_map: HashMap::new(),
            current_id: 1,
        }
    }

    pub fn create_future(
        &mut self,
        msg: &mut ButtplugMessageUnion,
    ) -> ClientConnectorMessageFuture {
        msg.set_id(self.current_id);
        let state = Arc::new(Mutex::new(ClientConnectorMessageState::default()));
        self.future_map.insert(self.current_id, state.clone());
        self.current_id += 1;
        let fut = ClientConnectorMessageFuture::new(&state);
        fut
    }

    pub fn resolve_message(&mut self, msg: &ButtplugMessageUnion) -> bool {
        match self.future_map.remove(&(msg.get_id())) {
            Some(_state) => {
                println!("found");
                let mut waker_state = _state.lock().unwrap();
                println!("making reply");
                waker_state.reply_msg = Some(msg.clone());
                println!("waking");
                match &waker_state.waker {
                    Some(_w) => {
                        let wake = waker_state.waker.take();
                        wake.unwrap().wake();
                    }
                    None => {
                        println!("No waker!");
                    }
                }
                true
            }
            None => {
                println!("Not found");
                false
            }
        }
    }
}
