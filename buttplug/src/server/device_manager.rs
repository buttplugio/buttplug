// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Buttplug Device Manager, manages Device Subtype (Platform/Communication bus
//! specific) Managers

use crate::core::errors::ButtplugError;

trait DeviceSubtypeManager {
    fn start_scanning() -> Result<(), ButtplugError>;
    fn stop_scanning() -> Result<(), ButtplugError>;
    fn is_scanning() -> bool;
}

// struct DeviceManager {}
