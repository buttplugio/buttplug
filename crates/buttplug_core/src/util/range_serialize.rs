use std::ops::RangeInclusive;

use serde::{Serializer, Serialize, ser::SerializeSeq};

pub fn option_range_serialize<S, T>(range: &Option<RangeInclusive<T>>, serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
  T: Serialize
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
  T: Serialize
{
  let mut seq = serializer.serialize_seq(Some(2))?;
  seq.serialize_element(&range.start())?;
  seq.serialize_element(&range.end())?;
  seq.end()
}

pub fn range_sequence_serialize<S,T>(
  range_vec: &Vec<RangeInclusive<T>>,
  serializer: S,
) -> Result<S::Ok, S::Error>
where
  S: Serializer,
  T: Serialize + Copy + Clone,
{
  let mut seq = serializer.serialize_seq(Some(range_vec.len()))?;
  for range in range_vec {
    seq.serialize_element(&vec![*range.start(), *range.end()])?;
  }
  seq.end()
}
