// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use serialport::SerialPortType;

#[derive(Debug, Clone)]
pub struct AvailableSerialPort {
  pub port_name: String,
  pub port_type: String,
  pub vid: Option<u16>,
  pub pid: Option<u16>,
  pub manufacturer: Option<String>,
  pub product: Option<String>,
  pub serial_number: Option<String>,
}

pub fn available_serial_ports() -> Vec<AvailableSerialPort> {
  match serialport::available_ports() {
    Ok(ports) => ports
      .into_iter()
      .map(|port| {
        let (port_type, vid, pid, manufacturer, product, serial_number) = match port.port_type {
          SerialPortType::UsbPort(info) => (
            "usb",
            Some(info.vid),
            Some(info.pid),
            info.manufacturer,
            info.product,
            info.serial_number,
          ),
          SerialPortType::PciPort => ("pci", None, None, None, None, None),
          SerialPortType::BluetoothPort => ("bluetooth", None, None, None, None, None),
          SerialPortType::Unknown => ("unknown", None, None, None, None, None),
        };
        AvailableSerialPort {
          port_name: port.port_name,
          port_type: port_type.to_owned(),
          vid,
          pid,
          manufacturer,
          product,
          serial_number,
        }
      })
      .collect(),
    Err(err) => {
      debug!("Failed to enumerate available serial ports: {}", err);
      vec![]
    }
  }
}
