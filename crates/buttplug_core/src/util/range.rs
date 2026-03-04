// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A range bounded inclusively below and above (`start..=end`).
/// Use this instead of `std::ops::RangeInclusive` when directly iterating over the range is not required.
/// It uses less memory and doesn't need special serialization.
#[derive(Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RangeInclusive<T: PartialOrd>([T; 2]);

impl<T: PartialOrd + Copy> RangeInclusive<T> {
  pub fn new(start: T, end: T) -> Self {
    Self([start, end])
  }

  pub fn start(&self) -> T {
    self.0[0]
  }

  pub fn end(&self) -> T {
    self.0[1]
  }

  pub fn is_empty(&self) -> bool {
    self.0[0] > self.0[1]
  }

  pub fn contains(&self, value: T) -> bool {
    value >= self.0[0] && value <= self.0[1]
  }
}

impl<T: PartialOrd + Copy> fmt::Debug for RangeInclusive<T>
where
  T: fmt::Debug,
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "[{:?}..={:?}]", self.0[0], self.0[1])
  }
}
