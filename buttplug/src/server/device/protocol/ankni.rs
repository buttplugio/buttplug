// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::server::device::configuration::ProtocolDeviceAttributes;
use crate::server::device::hardware::HardwareReadCmd;
use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    configuration::ProtocolAttributesType,
    hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
    protocol::{
      generic_protocol_initializer_setup,
      ProtocolHandler,
      ProtocolIdentifier,
      ProtocolInitializer,
    },
    ServerDeviceIdentifier,
  },
};
use async_trait::async_trait;
use std::sync::Arc;

generic_protocol_initializer_setup!(Ankni, "ankni");

#[derive(Default)]
pub struct AnkniInitializer {}

#[async_trait]
impl ProtocolInitializer for AnkniInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let msg = HardwareReadCmd::new(Endpoint::Generic0, 16, 100);
    let reading = hardware.read_value(&msg).await?;

    // No mac address on PnP characteristic, assume no handshake required
    if reading.data().len() > 6 {
      return Ok(Arc::new(Ankni::default()));
    }

    let mut addrdata = Vec::with_capacity(7);
    addrdata.push(0x01);
    addrdata.extend(reading.data());

    let check = ((crc16(addrdata) & 0xff00) >> 8) as u8;
    debug!("Ankni Checksum: {:#02X}", check);

    let msg = HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![
        0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
        0x01, 0x01, 0x01, 0x01, 0x01,
      ],
      true,
    );
    hardware.write_value(&msg).await?;
    let msg = HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![
        0x01, 0x02, check, check, check, check, check, check, check, check, check, check, check,
        check, check, check, check, check, 0x00, 0x00,
      ],
      true,
    );
    hardware.write_value(&msg).await?;
    Ok(Arc::new(Ankni::default()))
  }
}

#[derive(Default)]
pub struct Ankni {}

impl ProtocolHandler for Ankni {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_vibrate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![
        0x03,
        0x12,
        scalar as u8,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
      ],
      true,
    )
    .into()])
  }
}

fn crc16(advert_data: Vec<u8>) -> u16 {
  let mut remain: u16 = 0;
  for byte in advert_data {
    remain ^= (byte as u16) << 8;
    for _ in 0..8 {
      if (remain & (1 << (u16::BITS - 1))) != 0 {
        remain <<= 1;
        remain ^= 0x1021;
      } else {
        remain <<= 1;
      }
    }
  }
  remain
}
