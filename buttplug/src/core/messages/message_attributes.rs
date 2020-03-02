// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::Endpoint;
#[cfg(feature = "serialize_json")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct MessageAttributes {
    #[cfg_attr(
        feature = "serialize_json",
        serde(rename = "FeatureCount"),
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub feature_count: Option<u32>,
    #[cfg_attr(
        feature = "serialize_json",
        serde(rename = "StepCount"),
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub step_count: Option<Vec<u32>>,
    #[cfg_attr(
        feature = "serialize_json",
        serde(rename = "Endpoints"),
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub endpoints: Option<Vec<Endpoint>>,
    #[cfg_attr(
        feature = "serialize_json",
        serde(rename = "MaxDuration"),
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub max_duration: Option<Vec<u32>>,
    #[cfg_attr(
        feature = "serialize_json",
        serde(rename = "Patterns"),
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub patterns: Option<Vec<Vec<String>>>,
    #[cfg_attr(
        feature = "serialize_json",
        serde(rename = "ActuatorType"),
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub actuator_type: Option<Vec<String>>,
    // Never serialize this, its for internal use only
    #[cfg_attr(
        feature = "serialize_json",
        serde(rename = "FeatureOrder"),
        serde(skip_serializing)
    )]
    pub feature_order: Option<Vec<u32>>,
}