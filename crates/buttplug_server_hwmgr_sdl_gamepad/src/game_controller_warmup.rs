// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project
// root for full license information.

//! Engages GameController.framework from the main dispatch queue.
//!
//! GameController only begins tracking controllers when first touched on the
//! main thread of a process whose main run loop is serviced. SDL's joystick
//! backend initializes on this crate's dedicated SDL thread, so its
//! `[GCController controllers]` snapshot runs off-main and never engages the
//! framework: observers register, but no connect events are ever synthesized
//! or delivered, and enumeration stays empty in GUI hosts (reproduced against
//! SDL 3.4.16).
//!
//! `engage_from_main_queue` schedules a static global block on the main
//! dispatch queue that touches `[GCController controllers]`. Hosts with a
//! serviced main run loop (Flutter UI, NSApplication) run the block, the
//! framework engages, and SDL's already-registered observers start receiving
//! connect events. Headless hosts never service the main queue, so the block
//! simply never runs and behavior is unchanged; `dispatch_async` means there
//! is no deadlock risk either way.

use std::ffi::{c_char, c_int, c_void};

/// Block literal as laid out by the compiler-embedded blocks ABI. This block
/// captures nothing and is fully static, so `Block_copy` inside
/// `dispatch_async` is a no-op and no allocation or lifetime management is
/// needed.
#[repr(C)]
struct BlockLiteral {
  isa: *const c_void,
  flags: c_int,
  reserved: c_int,
  invoke: unsafe extern "C" fn(*const c_void),
  descriptor: *const BlockDescriptor,
}

#[repr(C)]
struct BlockDescriptor {
  reserved: usize,
  size: usize,
}

const BLOCK_IS_GLOBAL: c_int = 1 << 28;
const BLOCK_HAS_DESCRIPTOR: c_int = 1 << 29;

// Soundness: the static block is immutable after initialization; its invoke
// callback only performs Objective-C message sends, which are process-global
// and thread-agnostic, and `dispatch_async` copies global blocks by reference
// without allocation.
unsafe impl Sync for BlockLiteral {}

static BLOCK_DESCRIPTOR: BlockDescriptor = BlockDescriptor {
  reserved: 0,
  size: std::mem::size_of::<BlockLiteral>(),
};

static TOUCH_BLOCK: BlockLiteral = BlockLiteral {
  isa: &raw const _NSConcreteGlobalBlock as *const c_void,
  flags: BLOCK_IS_GLOBAL | BLOCK_HAS_DESCRIPTOR,
  reserved: 0,
  invoke: touch_game_controller,
  descriptor: &raw const BLOCK_DESCRIPTOR,
};

unsafe extern "C" {
  static _NSConcreteGlobalBlock: u8;
  /// The main queue object itself; `dispatch_get_main_queue()` is defined as
  /// its address, so pass `&raw const _dispatch_main_q` to dispatch calls.
  static _dispatch_main_q: u8;
  fn dispatch_async(queue: *mut c_void, block: *mut c_void);
  fn objc_getClass(name: *const c_char) -> *mut c_void;
  fn sel_registerName(name: *const c_char) -> *const c_void;
  fn objc_msgSend(
    receiver: *mut c_void,
    sel: *const c_void,
  ) -> *mut c_void;
}

/// Touching the `controllers` array is the point: it makes the framework
/// start its controller tracking. The returned array is left to the main
/// queue's autorelease pool.
unsafe extern "C" fn touch_game_controller(_block: *const c_void) {
  unsafe {
    let class = objc_getClass(b"GCController\0".as_ptr() as *const c_char);
    if class.is_null() {
      return;
    }
    let controllers = objc_msgSend(
      class,
      sel_registerName(b"controllers\0".as_ptr() as *const c_char),
    );
    let _ = controllers;
  }
}

/// Schedule the GameController touch on the main dispatch queue. One-shot per
/// process; repeat calls re-run the touch, which is harmless.
pub(crate) fn engage_from_main_queue() {
  unsafe {
    dispatch_async(
      &raw const _dispatch_main_q as *mut c_void,
      &raw const TOUCH_BLOCK as *mut c_void,
    );
  }
}
