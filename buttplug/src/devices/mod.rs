pub mod configuration_manager;
pub mod protocol;
pub mod protocols;
pub mod device;
#[cfg(feature = "serialize_json")]
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{fmt, str::FromStr, string::ToString};

#[derive(EnumString, Clone, Debug, PartialEq, Eq, Hash, Display, Copy)]
#[strum(serialize_all = "lowercase")]
pub enum Endpoint {
    Tx,
    Rx,
    Command,
    Firmware,
    TxMode,
    TxVibrate,
    TxShock,
    TxVendorControl,
    Whitelist,
}

impl Serialize for Endpoint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

struct EndpointVisitor;

impl<'de> Visitor<'de> for EndpointVisitor {
    type Value = Endpoint;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string representing an endpoint")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Endpoint::from_str(value).map_err(|e| E::custom(format!("{}", e)))
    }
}

impl<'de> Deserialize<'de> for Endpoint {
    fn deserialize<D>(deserializer: D) -> Result<Endpoint, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(EndpointVisitor)
    }
}
