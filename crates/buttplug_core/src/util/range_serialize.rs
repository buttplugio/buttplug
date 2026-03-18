// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::ops::RangeInclusive;

use serde::{Serialize, Serializer};

pub fn option_range_serialize<S, T>(
  range: &Option<RangeInclusive<T>>,
  serializer: S,
) -> Result<S::Ok, S::Error>
where
  S: Serializer,
  T: Serialize + Copy,
{
  if let Some(r) = range {
    range_serialize(r, serializer)
  } else {
    core::option::Option::None::<T>.serialize(serializer)
  }
}

pub fn range_serialize<S, T>(range: &RangeInclusive<T>, serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
  T: Serialize + Copy,
{
  [*range.start(), *range.end()].serialize(serializer)
}

pub fn range_sequence_serialize<S, T>(
  range_vec: &Vec<RangeInclusive<T>>,
  serializer: S,
) -> Result<S::Ok, S::Error>
where
  S: Serializer,
  T: Serialize + Copy,
{
  let arrays: Vec<[T; 2]> = range_vec.iter().map(|r| [*r.start(), *r.end()]).collect();
  arrays.serialize(serializer)
}
