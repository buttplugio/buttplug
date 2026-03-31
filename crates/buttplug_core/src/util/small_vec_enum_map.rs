//! A compact map-like collection for enum types with newtype variants.
//!
//! [`SmallVecEnumMap<V, N>`] stores enum values in a [`SmallVec`] and serializes/deserializes
//! them as a JSON object, where each key is the (snake_case) variant name and each value is
//! the inner payload. This preserves an existing `{"vibrate": {...}, "rotate": {...}}` wire
//! format while avoiding the memory overhead of a struct with one `Option` field per variant.
//!
//! With `N = 1`, no heap allocation is needed when a single variant is present — the common case.
//!
//! # Constraints
//! `V` must be an enum where every active variant is a newtype variant (single unnamed field).
//! Unit, tuple, and struct variants are not supported and will return a serde error at runtime.

use core::fmt;
use serde::{
  Deserialize,
  Deserializer,
  Serialize,
  Serializer,
  de::{MapAccess, Visitor},
  ser::SerializeMap,
};
use smallvec::SmallVec;
use std::ops::{Deref, DerefMut};

const UNSUPPORTED: &str = "SmallVecEnumMap only supports enums where every active variant is a \
                           newtype variant (single unnamed field). Unit, tuple, and struct \
                           variants are not supported. Check that your enum definition matches \
                           these constraints or see the small_vec_enum_map module docs for \
                           details.";

/// A [`SmallVec`]-backed collection that round-trips through a JSON object.
///
/// See the [module docs](self) for usage and constraints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmallVecEnumMap<V, const N: usize>(pub SmallVec<[V; N]>);

impl<V, const N: usize> Deref for SmallVecEnumMap<V, N> {
  type Target = SmallVec<[V; N]>;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<V, const N: usize> DerefMut for SmallVecEnumMap<V, N> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

impl<V, const N: usize> From<SmallVec<[V; N]>> for SmallVecEnumMap<V, N> {
  fn from(inner: SmallVec<[V; N]>) -> Self {
    SmallVecEnumMap(inner)
  }
}

impl<V, const N: usize> From<Vec<V>> for SmallVecEnumMap<V, N> {
  fn from(vec: Vec<V>) -> Self {
    SmallVecEnumMap(SmallVec::from_vec(vec))
  }
}

impl<V, const N: usize> FromIterator<V> for SmallVecEnumMap<V, N> {
  fn from_iter<I: IntoIterator<Item = V>>(iter: I) -> Self {
    SmallVecEnumMap(SmallVec::from_iter(iter))
  }
}

impl<V, const N: usize> SmallVecEnumMap<V, N> {
  pub fn is_empty(&self) -> bool {
    self.0.is_empty()
  }
}

impl<V, const N: usize> Default for SmallVecEnumMap<V, N> {
  fn default() -> Self {
    SmallVecEnumMap(SmallVec::new())
  }
}

/// Associates an enum type with a plain discriminant key that can be compared
/// without inspecting variant payloads or matching on string names.
///
/// Implement this on an enum to enable [`SmallVecEnumMap::find_by_key`] and
/// [`SmallVecEnumMap::contains_key`].
pub trait VariantKey {
  type Key: PartialEq;
  fn variant_key(&self) -> Self::Key;
}

impl<V, const N: usize> SmallVecEnumMap<V, N>
where
  V: VariantKey,
{
  /// Returns a reference to the element whose variant key equals `key`, if any.
  pub fn find_by_key(&self, key: &V::Key) -> Option<&V> {
    self.0.iter().find(|v| &v.variant_key() == key)
  }

  /// Returns `true` if an element with the given variant key is present.
  pub fn contains_key(&self, key: &V::Key) -> bool {
    self.0.iter().any(|v| &v.variant_key() == key)
  }
}

// ---------------------------------------------------------------------------
// Serialization
// ---------------------------------------------------------------------------

impl<V, const N: usize> Serialize for SmallVecEnumMap<V, N>
where
  V: Serialize,
{
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let mut map = serializer.serialize_map(Some(self.0.len()))?;
    for v in &self.0 {
      // Route each element through MapEntrySerializer, which intercepts the
      // serialize_newtype_variant call that serde generates for enum variants
      // and writes it directly as a map key-value pair.
      v.serialize(MapEntrySerializer { map: &mut map })?;
    }
    map.end()
  }
}

/// A [`Serializer`] adapter that writes a single enum newtype variant into an
/// outer [`SerializeMap`] as a key-value entry.
///
/// Only `serialize_newtype_variant` is meaningful here. All other methods return
/// an error. Serde has no `forward_to_error!` macro for serializers (unlike
/// `forward_to_deserialize_any!` for deserializers), so the full trait surface
/// must be spelled out below.
struct MapEntrySerializer<'a, M: SerializeMap> {
  map: &'a mut M,
}

/// Generates `Serializer` stub methods that unconditionally return [`UNSUPPORTED`].
///
/// Used for every path in [`MapEntrySerializer`] except `serialize_newtype_variant`.
/// Methods with generic type parameters or compound return types are written out
/// explicitly below since they can't fit this pattern cleanly.
macro_rules! reject_serialize {
  ($(fn $name:ident($($arg:tt)*);)*) => {$(
    fn $name(self, $($arg)*) -> Result<Self::Ok, Self::Error> {
      Err(serde::ser::Error::custom(UNSUPPORTED))
    }
  )*};
}

impl<'a, M> Serializer for MapEntrySerializer<'a, M>
where
  M: SerializeMap,
{
  type Ok = ();
  type Error = M::Error;
  type SerializeSeq = serde::ser::Impossible<(), M::Error>;
  type SerializeTuple = serde::ser::Impossible<(), M::Error>;
  type SerializeTupleStruct = serde::ser::Impossible<(), M::Error>;
  type SerializeTupleVariant = serde::ser::Impossible<(), M::Error>;
  type SerializeMap = serde::ser::Impossible<(), M::Error>;
  type SerializeStruct = serde::ser::Impossible<(), M::Error>;
  type SerializeStructVariant = serde::ser::Impossible<(), M::Error>;

  // Scalar and unit methods — all unsupported.
  reject_serialize! {
    fn serialize_bool(_v: bool);
    fn serialize_i8(_v: i8);
    fn serialize_i16(_v: i16);
    fn serialize_i32(_v: i32);
    fn serialize_i64(_v: i64);
    fn serialize_u8(_v: u8);
    fn serialize_u16(_v: u16);
    fn serialize_u32(_v: u32);
    fn serialize_u64(_v: u64);
    fn serialize_f32(_v: f32);
    fn serialize_f64(_v: f64);
    fn serialize_char(_v: char);
    fn serialize_str(_v: &str);
    fn serialize_bytes(_v: &[u8]);
    fn serialize_none();
    fn serialize_unit();
    fn serialize_unit_struct(_name: &'static str);
    fn serialize_unit_variant(_name: &'static str, _idx: u32, _variant: &'static str);
  }

  // Generic methods — can't go in the macro due to type parameter bounds.
  fn serialize_some<T: ?Sized + Serialize>(self, _value: &T) -> Result<Self::Ok, Self::Error> {
    Err(serde::ser::Error::custom(UNSUPPORTED))
  }
  fn serialize_newtype_struct<T: ?Sized + Serialize>(
    self,
    _name: &'static str,
    _value: &T,
  ) -> Result<Self::Ok, Self::Error> {
    Err(serde::ser::Error::custom(UNSUPPORTED))
  }

  /// The only supported path: writes the variant name as the map key and the
  /// inner value as the map value.
  fn serialize_newtype_variant<T: ?Sized + Serialize>(
    self,
    _name: &'static str,
    _variant_index: u32,
    variant: &'static str,
    value: &T,
  ) -> Result<Self::Ok, Self::Error> {
    self.map.serialize_entry(variant, value)
  }

  // Compound-return-type methods — can't go in the macro due to distinct return types.
  fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
    Err(serde::ser::Error::custom(UNSUPPORTED))
  }
  fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
    Err(serde::ser::Error::custom(UNSUPPORTED))
  }
  fn serialize_tuple_struct(
    self,
    _name: &'static str,
    _len: usize,
  ) -> Result<Self::SerializeTupleStruct, Self::Error> {
    Err(serde::ser::Error::custom(UNSUPPORTED))
  }
  fn serialize_tuple_variant(
    self,
    _name: &'static str,
    _variant_index: u32,
    _variant: &'static str,
    _len: usize,
  ) -> Result<Self::SerializeTupleVariant, Self::Error> {
    Err(serde::ser::Error::custom(UNSUPPORTED))
  }
  fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
    Err(serde::ser::Error::custom(UNSUPPORTED))
  }
  fn serialize_struct(
    self,
    _name: &'static str,
    _len: usize,
  ) -> Result<Self::SerializeStruct, Self::Error> {
    Err(serde::ser::Error::custom(UNSUPPORTED))
  }
  fn serialize_struct_variant(
    self,
    _name: &'static str,
    _variant_index: u32,
    _variant: &'static str,
    _len: usize,
  ) -> Result<Self::SerializeStructVariant, Self::Error> {
    Err(serde::ser::Error::custom(UNSUPPORTED))
  }
}

// ---------------------------------------------------------------------------
// Deserialization
//
// Pipeline for a single map entry (key string → enum variant):
//
//   SmallVecEnumMapVisitor::visit_map
//     └─ EnumEntrySeed        (carries the key string into value deserialization)
//          └─ EnumEntryDeserializer  (wraps the value deserializer; only accepts deserialize_enum)
//               └─ EnumAccess       (presents the key as the variant discriminant)
//                    └─ VariantAccess  (delegates inner-value deserialization to the original deserializer)
// ---------------------------------------------------------------------------

impl<'de, V, const N: usize> Deserialize<'de> for SmallVecEnumMap<V, N>
where
  V: Deserialize<'de>,
{
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    struct SmallVecEnumMapVisitor<V, const N: usize>(std::marker::PhantomData<[V; N]>);

    impl<'de, V, const N: usize> Visitor<'de> for SmallVecEnumMapVisitor<V, N>
    where
      V: Deserialize<'de>,
    {
      type Value = SmallVecEnumMap<V, N>;

      fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a map that can be converted into a SmallVecEnumMap")
      }

      fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
      where
        A: MapAccess<'de>,
      {
        let mut smallvec = SmallVec::new();
        while let Some(key) = map.next_key::<String>()? {
          let value = map.next_value_seed(EnumEntrySeed {
            key,
            _phantom: std::marker::PhantomData,
          })?;
          smallvec.push(value);
        }
        Ok(SmallVecEnumMap(smallvec))
      }
    }

    deserializer.deserialize_map(SmallVecEnumMapVisitor(std::marker::PhantomData))
  }
}

/// Carries a map key string into the value deserialization step so it can be
/// used as the enum variant discriminant.
struct EnumEntrySeed<V> {
  key: String,
  _phantom: std::marker::PhantomData<V>,
}

impl<'de, V> serde::de::DeserializeSeed<'de> for EnumEntrySeed<V>
where
  V: Deserialize<'de>,
{
  type Value = V;

  fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
  where
    D: Deserializer<'de>,
  {
    V::deserialize(EnumEntryDeserializer {
      key: self.key,
      value_deserializer: deserializer,
    })
  }
}

/// A [`Deserializer`] that only supports `deserialize_enum`.
///
/// Pairs the already-decoded key string with the not-yet-decoded value deserializer
/// so serde can reconstruct the enum variant.
struct EnumEntryDeserializer<D> {
  key: String,
  value_deserializer: D,
}

impl<'de, D> Deserializer<'de> for EnumEntryDeserializer<D>
where
  D: Deserializer<'de>,
{
  type Error = D::Error;

  fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    Err(serde::de::Error::custom(UNSUPPORTED))
  }

  fn deserialize_enum<V>(
    self,
    _name: &'static str,
    _variants: &'static [&'static str],
    visitor: V,
  ) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    visitor.visit_enum(EnumAccess {
      key: self.key,
      value_deserializer: self.value_deserializer,
    })
  }

  serde::forward_to_deserialize_any! {
    bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
    bytes byte_buf option unit unit_struct newtype_struct seq tuple
    tuple_struct map struct identifier ignored_any
  }
}

/// Presents the map key as the enum variant discriminant to serde's enum visitor.
struct EnumAccess<D> {
  key: String,
  value_deserializer: D,
}

impl<'de, D> serde::de::EnumAccess<'de> for EnumAccess<D>
where
  D: Deserializer<'de>,
{
  type Error = D::Error;
  type Variant = VariantAccess<D>;

  fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
  where
    V: serde::de::DeserializeSeed<'de>,
  {
    let variant = seed.deserialize(serde::de::value::StringDeserializer::new(self.key))?;
    Ok((
      variant,
      VariantAccess {
        value_deserializer: self.value_deserializer,
      },
    ))
  }
}

/// Delegates deserialization of the variant's inner value to the original deserializer.
///
/// Only newtype variants are supported; all other variant forms return an error.
struct VariantAccess<D> {
  value_deserializer: D,
}

impl<'de, D> serde::de::VariantAccess<'de> for VariantAccess<D>
where
  D: Deserializer<'de>,
{
  type Error = D::Error;

  fn unit_variant(self) -> Result<(), Self::Error> {
    Err(serde::de::Error::custom(UNSUPPORTED))
  }

  fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
  where
    T: serde::de::DeserializeSeed<'de>,
  {
    seed.deserialize(self.value_deserializer)
  }

  fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    Err(serde::de::Error::custom(UNSUPPORTED))
  }

  fn struct_variant<V>(
    self,
    _fields: &'static [&'static str],
    _visitor: V,
  ) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    Err(serde::de::Error::custom(UNSUPPORTED))
  }
}
