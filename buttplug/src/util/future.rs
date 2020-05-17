// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Buttplug futures utilities. Mostly used for building message futures in the
//! client, used to wait on responses from the server.

use async_std::{
  future::Future,
  task::{Context, Poll, Waker},
};
use core::pin::Pin;
use std::sync::{Arc, Mutex};

/// Struct used for facilitating passing futures across channels.
///
/// There are quite a few times within Buttplug where we will need to pass a
/// future between tasks. For instance, when a ButtplugMessage is sent to the
/// server, it may take an indeterminate amount of time to get a reply, and we
/// may have to traverse 2-3 tasks to make this happen. This struct holds the
/// reply, as well as a [Waker] for the related future. Once the reply is
/// filled, the waker will be called to finish the future polling.
#[derive(Debug, Clone)]
pub struct ButtplugFutureState<T> {
  reply: Option<T>,
  waker: Option<Waker>,
}

// For some reason, deriving default above doesn't work, but doing an explicit
// derive here does work.
impl<T> Default for ButtplugFutureState<T> {
  fn default() -> Self {
    ButtplugFutureState::<T> {
      reply: None,
      waker: None,
    }
  }
}

impl<T> ButtplugFutureState<T> {
  /// Sets the response for the future, firing the waker.
  ///
  /// When a response is received from whatever we're waiting on, this function
  /// takes the response, updates the state struct, and calls [Waker::wake] so
  /// that the corresponding future can finish.
  pub fn set_reply(&mut self, reply: T) {
    if self.reply.is_some() {
      panic!("set_reply_msg called multiple times on the same future.");
    }

    self.reply = Some(reply);

    if self.waker.is_some() {
      self.waker.take().unwrap().wake();
    }
  }
}

/// Shared [ButtplugFutureState] type.
///
/// [ButtplugFutureState] is made to be shared across futures, and we'll
/// never know if those futures are single or multithreaded. Only needs to
/// unlock for calls to [ButtplugFutureState::set_reply].
pub type ButtplugFutureStateShared<T> = Arc<Mutex<ButtplugFutureState<T>>>;

/// [Future] implementation for long operations in Buttplug.
///
/// This is a convenience struct, used for handling indeterminately long
/// operations, like Buttplug's request/reply communications between the client
/// and server. It allows us to say what type we expect back, then hold a waker
/// that we can pass around as needed.
#[derive(Debug)]
pub struct ButtplugFuture<T> {
  /// State that holds the waker for the future, and the reply (once set).
  ///
  /// ## Notes
  ///
  /// This needs to be an [Arc]<[Mutex]<T>> in order to make it mutable under
  /// pinning when dealing with being a future. There is a chance we could do
  /// this as a [Pin::get_unchecked_mut] borrow, which would be way faster, but
  /// that's dicey and hasn't been proven as needed for speed yet.
  waker_state: ButtplugFutureStateShared<T>,
}

// TODO Should we implement drop on this?
//
// It'd be nice if the future would yell if its dropping and the waker didn't
// fire? Otherwise it seems like we could have quiet deadlocks.

impl<T> Default for ButtplugFuture<T> {
  fn default() -> Self {
    ButtplugFuture::<T> {
      waker_state: ButtplugFutureStateShared::<T>::default(),
    }
  }
}

impl<T> ButtplugFuture<T> {
  /// Returns a clone of the state, used for moving the state across contexts
  /// (tasks/threads/etc...).
  pub fn get_state_clone(&self) -> ButtplugFutureStateShared<T> {
    self.waker_state.clone()
  }
}

impl<T> Future for ButtplugFuture<T> {
  type Output = T;

  /// Wakes up when the Output type reply has been set in the
  /// [ButtplugFutureStateShared].
  fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
    let mut waker_state = self.waker_state.lock().unwrap();
    if waker_state.reply.is_some() {
      let msg = waker_state.reply.take().unwrap();
      Poll::Ready(msg)
    } else {
      debug!("Waker set.");
      waker_state.waker = Some(cx.waker().clone());
      Poll::Pending
    }
  }
}
