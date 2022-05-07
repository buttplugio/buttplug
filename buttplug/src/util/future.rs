// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Buttplug futures utilities. Mostly used for building message futures in the
//! client, used to wait on responses from the server.

use core::pin::Pin;
use futures::{
  future::Future,
  task::{Context, Poll, Waker},
};
use std::sync::{Arc, Mutex, MutexGuard};

/// Struct used for facilitating resolving futures across contexts.
///
/// Since ButtplugFuture is [Pinned][Pin], we can't just go passing it around
/// tasks or threads. This struct is therefore used to get replies from other
/// contexts while letting the future stay pinned. It holds the reply to the
/// future, as well as a [Waker] for waking up the future when the reply is set.
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
  ///
  /// # Panics
  ///
  /// If the reply is set twice, the library will panic. We have no way of
  /// resolving two replies to the same future, so this is considered a
  /// catastrophic error.
  pub fn set_reply(&mut self, reply: T) {
    if self.reply.is_some() {
      panic!("set_reply_msg called multiple times on the same future.");
    }

    self.reply = Some(reply);

    if self.waker.is_some() {
      self.waker.take().expect("Already checked validity").wake();
    }
  }
}

/// Shared [ButtplugFutureState] type.
///
/// [ButtplugFutureState] is made to be shared across tasks, and we'll never
/// know if those tasks are running on single or multithreaded executors.
///
/// # Panics and notes on setting replies
///
/// The lock for a [ButtplugFutureState] should only ever be taken when the
/// reply is being set (which the `set_reply` method does internally), and there
/// should never be a point where the reply is set twice (See the panic
/// documentation for [ButtplugFutureState]). In order to make sure we never
/// block, we always lock using try_lock with .expect(). If try_lock fails, this
/// means we're already in a double reply situation, and therefore we'll panic
/// on the .expect(). Any panic from this should be considered a library error
/// and reported as a bug.
#[derive(Debug)]
pub struct ButtplugFutureStateShared<T> {
  /// The internal state of the future. When `set_reply` is run, we fill this in
  /// with the value we want the related future to resolve with.
  state: Arc<Mutex<ButtplugFutureState<T>>>,
}

impl<T> ButtplugFutureStateShared<T> {
  pub fn new(state: ButtplugFutureState<T>) -> Self {
    Self {
      state: Arc::new(Mutex::new(state)),
    }
  }

  /// Locks and returns a [MutexGuard].
  ///
  /// See [ButtplugFutureStateShared] struct documentation for more info on
  /// locking.
  ///
  /// # Visibility
  ///
  /// The only thing that needs to read the reply from a future is our poll
  /// method, in this module. Everything else should just be setting replies,
  /// and can use set_reply accordingly.
  pub(super) fn lock(&self) -> MutexGuard<'_, ButtplugFutureState<T>> {
    self
      .state
      .lock()
      .expect("There should never be lock contention for a buttplug future.")
  }

  /// Locks immediately and sets the reply for the internal waker, or panics if
  /// lock is held.
  ///
  /// See [ButtplugFutureStateShared] struct documentation for more info on
  /// locking.
  pub fn set_reply(&self, reply: T) {
    self.lock().set_reply(reply);
  }
}

impl<T> Default for ButtplugFutureStateShared<T> {
  fn default() -> Self {
    Self {
      state: Arc::new(Mutex::new(ButtplugFutureState::<T>::default())),
    }
  }
}

impl<T> Clone for ButtplugFutureStateShared<T> {
  fn clone(&self) -> Self {
    Self {
      state: self.state.clone(),
    }
  }
}

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
    // This is the only place lock_now_or_panic should be called, since we're
    // reading the value.
    let mut waker_state = self.waker_state.lock();
    if waker_state.reply.is_some() {
      let msg = waker_state.reply.take().expect("Already checked validity");
      Poll::Ready(msg)
    } else {
      waker_state.waker = Some(cx.waker().clone());
      Poll::Pending
    }
  }
}
