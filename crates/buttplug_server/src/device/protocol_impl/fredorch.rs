// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareEvent, HardwareSubscribeCmd, HardwareWriteCmd},
  protocol::{
    ProtocolHandler,
    ProtocolIdentifier,
    ProtocolInitializer,
    generic_protocol_initializer_setup,
  },
};
use async_trait::async_trait;
use buttplug_core::{errors::ButtplugDeviceError, util::sleep};
use buttplug_server_device_config::{
  Endpoint,
  ProtocolCommunicationSpecifier,
  ServerDeviceDefinition,
  UserDeviceIdentifier,
};
use futures::FutureExt;
use std::{
  sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
  },
  time::Duration,
};
use tokio::select;
use uuid::{Uuid, uuid};

const FREDORCH_COMMAND_TIMEOUT_MS: u64 = 500;

const CRC_HI: [u8; 256] = [
  0, 193, 129, 64, 1, 192, 128, 65, 1, 192, 128, 65, 0, 193, 129, 64, 1, 192, 128, 65, 0, 193, 129,
  64, 0, 193, 129, 64, 1, 192, 128, 65, 1, 192, 128, 65, 0, 193, 129, 64, 0, 193, 129, 64, 1, 192,
  128, 65, 0, 193, 129, 64, 1, 192, 128, 65, 1, 192, 128, 65, 0, 193, 129, 64, 1, 192, 128, 65, 0,
  193, 129, 64, 0, 193, 129, 64, 1, 192, 128, 65, 0, 193, 129, 64, 1, 192, 128, 65, 1, 192, 128,
  65, 0, 193, 129, 64, 0, 193, 129, 64, 1, 192, 128, 65, 1, 192, 128, 65, 0, 193, 129, 64, 1, 192,
  128, 65, 0, 193, 129, 64, 0, 193, 129, 64, 1, 192, 128, 65, 1, 192, 128, 65, 0, 193, 129, 64, 0,
  193, 129, 64, 1, 192, 128, 65, 0, 193, 129, 64, 1, 192, 128, 65, 1, 192, 128, 65, 0, 193, 129,
  64, 0, 193, 129, 64, 1, 192, 128, 65, 1, 192, 128, 65, 0, 193, 129, 64, 1, 192, 128, 65, 0, 193,
  129, 64, 0, 193, 129, 64, 1, 192, 128, 65, 0, 193, 129, 64, 1, 192, 128, 65, 1, 192, 128, 65, 0,
  193, 129, 64, 1, 192, 128, 65, 0, 193, 129, 64, 0, 193, 129, 64, 1, 192, 128, 65, 1, 192, 128,
  65, 0, 193, 129, 64, 0, 193, 129, 64, 1, 192, 128, 65, 0, 193, 129, 64, 1, 192, 128, 65, 1, 192,
  128, 65, 0, 193, 129, 64,
];
const CRC_LO: [u8; 256] = [
  0, 192, 193, 1, 195, 3, 2, 194, 198, 6, 7, 199, 5, 197, 196, 4, 204, 12, 13, 205, 15, 207, 206,
  14, 10, 202, 203, 11, 201, 9, 8, 200, 216, 24, 25, 217, 27, 219, 218, 26, 30, 222, 223, 31, 221,
  29, 28, 220, 20, 212, 213, 21, 215, 23, 22, 214, 210, 18, 19, 211, 17, 209, 208, 16, 240, 48, 49,
  241, 51, 243, 242, 50, 54, 246, 247, 55, 245, 53, 52, 244, 60, 252, 253, 61, 255, 63, 62, 254,
  250, 58, 59, 251, 57, 249, 248, 56, 40, 232, 233, 41, 235, 43, 42, 234, 238, 46, 47, 239, 45,
  237, 236, 44, 228, 36, 37, 229, 39, 231, 230, 38, 34, 226, 227, 35, 225, 33, 32, 224, 160, 96,
  97, 161, 99, 163, 162, 98, 102, 166, 167, 103, 165, 101, 100, 164, 108, 172, 173, 109, 175, 111,
  110, 174, 170, 106, 107, 171, 105, 169, 168, 104, 120, 184, 185, 121, 187, 123, 122, 186, 190,
  126, 127, 191, 125, 189, 188, 124, 180, 116, 117, 181, 119, 183, 182, 118, 114, 178, 179, 115,
  177, 113, 112, 176, 80, 144, 145, 81, 147, 83, 82, 146, 150, 86, 87, 151, 85, 149, 148, 84, 156,
  92, 93, 157, 95, 159, 158, 94, 90, 154, 155, 91, 153, 89, 88, 152, 136, 72, 73, 137, 75, 139,
  138, 74, 78, 142, 143, 79, 141, 77, 76, 140, 68, 132, 133, 69, 135, 71, 70, 134, 130, 66, 67,
  131, 65, 129, 128, 64,
];
pub fn crc16(data: &[u8]) -> [u8; 2] {
  let mut n: u8 = 255;
  let mut o: u8 = 255;
  for val in data {
    let a: u8 = n ^ val;
    n = o ^ CRC_HI[a as usize];
    o = CRC_LO[a as usize]
  }
  [n, o]
}

const FREDORCH_PROTOCOL_UUID: Uuid = uuid!("f9a83f46-0af5-4766-84f0-a1cca6614115");

generic_protocol_initializer_setup!(Fredorch, "fredorch");

#[derive(Default)]
pub struct FredorchInitializer {}

#[async_trait]
impl ProtocolInitializer for FredorchInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let mut event_receiver = hardware.event_stream();
    hardware
      .subscribe(&HardwareSubscribeCmd::new(
        FREDORCH_PROTOCOL_UUID,
        Endpoint::Rx,
      ))
      .await?;

    let init: Vec<(String, Vec<u8>)> = vec![
      (
        "Set the device to program mode".to_owned(),
        vec![0x01, 0x06, 0x00, 0x64, 0x00, 0x01],
      ),
      (
        "Set the program mode to record".to_owned(),
        vec![0x01, 0x06, 0x00, 0x69, 0x00, 0x00],
      ),
      (
        "Program the device to move to position 0 at speed 5".to_owned(),
        vec![
          0x01, 0x10, 0x00, 0x6b, 0x00, 0x05, 0x0a, 0x00, 0x05, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00,
          0x00, 0x01,
        ],
      ),
      (
        "Run the program".to_owned(),
        vec![0x01, 0x06, 0x00, 0x69, 0x00, 0x01],
      ),
      (
        "Set the program to repeat".to_owned(),
        vec![0x01, 0x06, 0x00, 0x6a, 0x00, 0x01],
      ),
    ];

    // expect 0, 1, 0, 1, 1 on connect
    select! {
      event = event_receiver.recv().fuse() => {
        if let Ok(HardwareEvent::Notification(_, _, n)) = event {
          debug!("Fredorch: wake up - received {:?}", n);
        } else {
          return Err(
            ButtplugDeviceError::ProtocolSpecificError(
              "Fredorch".to_owned(),
              "Fredorch Device disconnected while initialising.".to_owned(),
            )
          );
        }
      }
      _ = sleep(Duration::from_millis(FREDORCH_COMMAND_TIMEOUT_MS)).fuse() => {
        // Or not?
      }
    }

    for mut data in init {
      let crc = crc16(&data.1);
      data.1.push(crc[0]);
      data.1.push(crc[1]);
      debug!("Fredorch: {} - sent {:?}", data.0, data.1);
      hardware
        .write_value(&HardwareWriteCmd::new(
          &[FREDORCH_PROTOCOL_UUID],
          Endpoint::Tx,
          data.1.clone(),
          false,
        ))
        .await?;

      select! {
        event = event_receiver.recv().fuse() => {
          if let Ok(HardwareEvent::Notification(_, _, n)) = event {
            debug!("Fredorch: {} - received {:?}", data.0, n);
          } else {
            return Err(
              ButtplugDeviceError::ProtocolSpecificError(
                "Fredorch".to_owned(),
                "Fredorch Device disconnected while initialising.".to_owned(),
              )
            );
          }
        }
        _ = sleep(Duration::from_millis(FREDORCH_COMMAND_TIMEOUT_MS)).fuse() => {
          return Err(
              ButtplugDeviceError::ProtocolSpecificError(
                "Fredorch".to_owned(),
                "Fredorch Device timed out while initialising.".to_owned(),
              )
            );
        }
      }
    }

    Ok(Arc::new(Fredorch::default()))
  }
}

const SPEED_MATRIX: [[u32; 20]; 15] = [
// distance, speed 1-20
/*      1     2     3     4     5     6     7     8     9    10   11   12   13   14   15   16   17   18   19   20 */
/* 1*/ [1000, 800,  400,  235,  200,  172,  155,  92,   60,  45,  38,  34,  32,  28,  27,  26,  25,  24,  23,  22 ],
/* 2*/ [1500, 1000, 800,  680,  600,  515,  425,  265,  165, 115, 80,  70,  50,  48,  45,  35,  34,  33,  32,  30 ],
/* 3*/ [2500, 2310, 1135, 925,  792,  695,  565,  380,  218, 155, 105, 82,  70,  68,  65,  60,  48,  45,  43,  40 ],
/* 4*/ [3000, 2800, 1500, 1155, 965,  810,  690,  465,  260, 195, 140, 110, 85,  75,  74,  73,  70,  65,  60,  55 ],
/* 5*/ [3400, 3232, 2305, 1380, 1200, 1165, 972,  565,  328, 235, 162, 132, 98,  78,  75,  74,  73,  72,  71,  70 ],
/* 6*/ [3500, 3350, 2500, 1640, 1250, 1210, 1010, 645,  385, 275, 175, 160, 115, 95,  91,  90,  85,  80,  77,  75 ],
/* 7*/ [3600, 3472, 2980, 2060, 1560, 1275, 1132, 738,  430, 310, 230, 170, 128, 122, 110, 108, 105, 103, 101, 100],
/* 8*/ [3800, 3500, 3055, 2105, 1740, 1370, 1290, 830,  490, 355, 235, 195, 150, 140, 135, 132, 130, 125, 120, 119],
/* 9*/ [3900, 3518, 3190, 2315, 2045, 1510, 1442, 1045, 552, 392, 280, 225, 172, 145, 140, 138, 135, 134, 132, 130],
/*10*/ [6000, 5755, 3240, 2530, 2135, 1605, 1500, 1200, 595, 425, 285, 245, 175, 170, 160, 155, 150, 145, 142, 140],
/*11*/ [6428, 5872, 3335, 2780, 2270, 1782, 1590, 1310, 648, 470, 315, 255, 182, 180, 175, 172, 170, 162, 160, 155],
/*12*/ [6730, 5950, 3490, 2995, 2395, 1890, 1650, 1350, 700, 500, 350, 290, 220, 190, 185, 180, 175, 170, 165, 160],
/*13*/ [6962, 6122, 3880, 3205, 2465, 1900, 1700, 1400, 835, 545, 375, 310, 228, 195, 190, 185, 182, 181, 180, 175],
/*14*/ [7945, 6365, 4130, 3470, 2505, 1910, 1755, 1510, 855, 580, 400, 330, 235, 210, 205, 200, 195, 190, 185, 180],
/*15*/ [8048, 7068, 4442, 3708, 2668, 1930, 1800, 1520, 878, 618, 428, 365, 260, 255, 250, 240, 230, 220, 210, 200],
];

fn calculate_speed(distance: u32, duration: u32) -> u8 {
  let distance = distance.clamp(0,15);
  if distance == 0  {return 0;}

  let mut speed= 1;
  while speed < 20 {
    if SPEED_MATRIX[distance as usize - 1][speed as usize - 1] < duration {
      return speed;
    }
    speed += 1;
  }
  speed
}

#[derive(Default)]
pub struct Fredorch {
  previous_position: Arc<AtomicU8>,
}

impl ProtocolHandler for Fredorch {
  fn handle_position_with_duration_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    position: u32,
    duration: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let previous_position = self.previous_position.load(Ordering::Relaxed);
    let distance = (previous_position as i32 - position as i32).abs() as u32;
    // The Fredorch only has 15 positions, but scales them to 0-150
    let pos = (position * 10) as u8;

    let speed = calculate_speed(distance, duration);
    let mut data: Vec<u8> = vec![
      0x01, 0x10, 0x00, 0x6B, 0x00, 0x05, 0x0a, 0x00, speed, 0x00, speed, 0x00, pos, 0x00, pos,
      0x00, 0x01,
    ];
    let crc = crc16(&data);
    data.push(crc[0]);
    data.push(crc[1]);
    self.previous_position.store(position as u8, Ordering::Relaxed);
    Ok(vec![HardwareWriteCmd::new(
      &[FREDORCH_PROTOCOL_UUID],
      Endpoint::Tx,
      data,
      false,
    )
    .into()])
  }
  
  // TODO: Something is off... I think we need to program in both directions independently
  fn handle_output_oscillate_cmd(&self, _feature_index: u32, _feature_id: Uuid, speed: u32) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    // If we ever get oscillate with range, these should be loaded from the last set range
    let min_pos = if speed == 0 { 0 } else { 0 };
    let max_pos = if speed == 0 { 0 } else { 15 };
    let mut data: Vec<u8> = vec![
      0x01, 0x10, 0x00, 0x6B, 0x00, 0x05, 0x0a, 0x00, speed as u8, 0x00, speed as u8, 0x00, min_pos * 15, 0x00, max_pos * 15,
      0x00, 0x01,
    ];
    let crc = crc16(&data);
    data.push(crc[0]);
    data.push(crc[1]);
    Ok(vec![HardwareWriteCmd::new(
      &[FREDORCH_PROTOCOL_UUID],
      Endpoint::Tx,
      data,
      false,
    )
        .into()])
  }
}
