// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
#[cfg(feature = "serialize_json")]
use serde::{Deserialize, Serialize};

/// Represents the Buttplug Protocol Ok message, as documented in the [Buttplug
/// Protocol Spec](https://buttplug-spec.docs.buttplug.io/status.html#ok).
#[derive(Debug, PartialEq, Default, ButtplugMessage, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct Ok {
    /// Message Id, used for matching message pairs in remote connection instances.
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub(super) id: u32,
}

impl Ok {
    /// Creates a new Ok message with the given Id.
    pub fn new(id: u32) -> Self {
        Self { id }
    }
}

#[cfg(feature = "serialize_json")]
#[cfg(test)]
mod test {
    use crate::core::messages::{ButtplugMessage, ButtplugMessageUnion, Ok};

    const OK_STR: &str = "{\"Ok\":{\"Id\":0}}";
    
    #[test]
    fn test_ok_serialize() {
        let ok = ButtplugMessageUnion::Ok(Ok::new(0));
        let js = serde_json::to_string(&ok).unwrap();
        assert_eq!(OK_STR, js);
    }

    #[test]
    fn test_protocol_json() {
        const PROTOCOL_STR: &str = "[{\"Ok\":{\"Id\":0}}]";
        let ok = ButtplugMessageUnion::Ok(Ok::new(0));
        let js = ok.as_protocol_json();
        assert_eq!(PROTOCOL_STR, js);
    }

    #[test]
    fn test_ok_deserialize() {
        let union: ButtplugMessageUnion = serde_json::from_str(&OK_STR).unwrap();
        assert_eq!(ButtplugMessageUnion::Ok(Ok::new(0)), union);
    }
}