//! Reusable serde helper modules for types that need custom serialization.

/// Serializes / deserializes `BitFlags<T>` as a sequence of variant name strings,
/// matching the wire format that `HashSet<T>` produced previously.
///
/// Usage: `#[serde(with = "crate::util::serializers::bitflags_seq")]`
pub mod bitflags_seq {
  use enumflags2::{BitFlag, BitFlags};
  use serde::{
    Deserialize,
    Deserializer,
    Serialize,
    Serializer,
    de::{SeqAccess, Visitor},
    ser::SerializeSeq,
  };
  use std::{fmt, marker::PhantomData};

  pub fn serialize<S, T>(flags: &BitFlags<T>, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
    T: BitFlag + Serialize,
  {
    let mut seq = serializer.serialize_seq(Some(flags.len()))?;
    for flag in flags.iter() {
      seq.serialize_element(&flag)?;
    }
    seq.end()
  }

  pub fn deserialize<'de, D, T>(deserializer: D) -> Result<BitFlags<T>, D::Error>
  where
    D: Deserializer<'de>,
    T: BitFlag + Deserialize<'de>,
  {
    struct FlagsVisitor<T>(PhantomData<T>);

    impl<'de, T> Visitor<'de> for FlagsVisitor<T>
    where
      T: BitFlag + Deserialize<'de>,
    {
      type Value = BitFlags<T>;

      fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a sequence of flag variant names")
      }

      fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
      where
        A: SeqAccess<'de>,
      {
        let mut flags = BitFlags::empty();
        while let Some(flag) = seq.next_element::<T>()? {
          flags |= flag;
        }
        Ok(flags)
      }
    }

    deserializer.deserialize_seq(FlagsVisitor(PhantomData))
  }
}
