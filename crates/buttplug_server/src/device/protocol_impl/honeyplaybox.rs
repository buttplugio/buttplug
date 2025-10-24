// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareEvent, HardwareSubscribeCmd, HardwareWriteCmd},
  protocol::{
    ProtocolHandler,
    ProtocolIdentifier,
    ProtocolInitializer,
    ProtocolKeepaliveStrategy,
    generic_protocol_initializer_setup,
  },
};
use async_trait::async_trait;
use buttplug_core::{errors::ButtplugDeviceError, message, util::sleep};
use buttplug_server_device_config::Endpoint;
use buttplug_server_device_config::{
  ProtocolCommunicationSpecifier,
  ServerDeviceDefinition,
  UserDeviceIdentifier,
};
use futures::FutureExt;
use md5::{Digest, Md5};
use std::{
  sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
  },
  time::Duration,
};
use futures_util::future::BoxFuture;
use tokio::select;
use uuid::{Uuid, uuid};
use buttplug_core::message::{InputData, InputReadingV4, InputTypeData};
use crate::device::hardware::HardwareReadCmd;

const HONEY_PLAYBOX_VIBRATE_INTERVAL: u64 = 5;
const HONEY_PLAYBOX_COMMAND_RETRY: u64 = 3;
const HONEY_PLAYBOX_PROTOCOL_UUID: Uuid = uuid!("0d1598bd-6845-4950-8aa0-416b1115fc7c");

// The secret key used for MD5 signing.
const SECRET: [u8; 16] = [
  0x8b, 0xe3, 0xfd, 0x04, 0x68, 0x35, 0x09, 0x86, 0x12, 0x1a, 0xbf, 0x03, 0x30, 0xe9, 0xe3, 0xc5,
];

generic_protocol_initializer_setup!(HoneyPlayBox, "honeyplaybox");

#[derive(Default)]
pub struct HoneyPlayBoxInitializer {}

#[async_trait]
impl ProtocolInitializer for HoneyPlayBoxInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    device_definition: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let feature_count = device_definition
      .features()
      .iter()
      .filter(|x| x.output().is_some())
      .count();

    let mut event_receiver = hardware.event_stream();
    hardware
      .subscribe(&HardwareSubscribeCmd::new(
        HONEY_PLAYBOX_PROTOCOL_UUID,
        Endpoint::Rx,
      ))
      .await?;

    let mut collector = FrameCollector::new();
    let mut count = 0;

    loop {
      hardware
        .write_value(&HardwareWriteCmd::new(
          &[HONEY_PLAYBOX_PROTOCOL_UUID],
          Endpoint::Tx,
          FrameCodec::build_frame(0xB1, 0x01, &[0x00, 0x10], 0x0B),
          true,
        ))
        .await?;

      // loop reads because the response is over multiple bluetooth packets
      for _ in 0..3 {
      select! {
        event = event_receiver.recv().fuse() => {
          if let Ok(HardwareEvent::Notification(_, _, payload)) = event {
            let frames = collector.push_bytes(&payload);
            for frame in frames {
                if let Some(packet) = FrameCodec::parse_response_frame(&frame) {
                    match packet.response_code {
                        Some(0x9000) => {
                            trace!("HoneyPlayBox handshake success on attempt {}", count+1);
                            if let Some(rand) = FrameCodec::extract_random(&frame) {
                                return Ok(Arc::new(HoneyPlayBox::new(rand, feature_count)));
                            }
                            warn!("HoneyPlayBox: No random token in frame (handshake attempt {})", count+1);
                        }
                        Some(code) => {
                            let msg = format!("Handshake failed, code={:04X}", code);
                            error!("HoneyPlayBox: {}", msg);
                            return Err(ButtplugDeviceError::ProtocolSpecificError(
                                "HoneyPlayBox".into(),
                                msg,
                            ));
                        }
                        None => warn!("HoneyPlayBox: No response code in frame (handshake attempt {})", count+1),
                    }
                }
            }
          } else {
            return Err(
              ButtplugDeviceError::ProtocolSpecificError(
                "HoneyPlayBox".to_owned(),
                "HoneyPlayBox Device disconnected during handshake.".to_owned(),
              ),
            );
          }
        }
        _ = sleep(Duration::from_secs(count+1)).fuse() => {
          count += 1;
          if count > HONEY_PLAYBOX_COMMAND_RETRY {
            error!("HoneyPlayBox Device timed out while waiting for handshake. ({} retries)", HONEY_PLAYBOX_COMMAND_RETRY);
            return Err(ButtplugDeviceError::ProtocolSpecificError(
                "HoneyPlayBox".into(),
                format!("Handshake failed after {} retries", HONEY_PLAYBOX_COMMAND_RETRY),
            ));
          }
        }
        }
      }
    }
  }
}

pub struct HoneyPlayBox {
  random_key: [u8; 16],
  last_command: Arc<Vec<AtomicU8>>,
}

impl HoneyPlayBox {
  fn new(random_key: [u8; 16], feature_count: usize) -> Self {
    Self {
      random_key,
      last_command: Arc::new(
        (0..feature_count)
          .map(|_| AtomicU8::new(0))
          .collect::<Vec<AtomicU8>>(),
      ),
    }
  }
}

#[async_trait]
impl ProtocolHandler for HoneyPlayBox {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    ProtocolKeepaliveStrategy::RepeatLastPacketStrategyWithTiming(Duration::from_secs(
      HONEY_PLAYBOX_VIBRATE_INTERVAL,
    ))
  }

  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.last_command[feature_index as usize].store(speed as u8, Ordering::Relaxed);
    let mut groups = vec![];
    for i in 0..self.last_command.len() {
      groups.push(VibrateGroup {
        work_mode: 1,
        motor_pos: i as u8,
        time_100ms: 60,
        freq: 0,
        strength: self.last_command[i].load(Ordering::Relaxed),
      });
    }

    let payload = build_vibrate_data(&self.random_key, &groups)
      .map_err(|e| ButtplugDeviceError::ProtocolSpecificError("HoneyPlayBox".into(), e))?;
    let data = FrameCodec::build_frame(0xB1, 0x03, &payload, 0x10);

    Ok(vec![
      HardwareWriteCmd::new(&[HONEY_PLAYBOX_PROTOCOL_UUID], Endpoint::Tx, data, true).into(),
    ])
  }


  fn handle_battery_level_cmd(
    &self,
    device_index: u32,
    device: Arc<Hardware>,
    feature_index: u32,
    feature_id: Uuid,
  ) -> BoxFuture<'_, Result<InputReadingV4, ButtplugDeviceError>> {
    debug!("Trying to get battery reading.");
    let msg = HardwareReadCmd::new(feature_id, Endpoint::RxBLEBattery, 20, 0);
    let fut = device.read_value(&msg);
    async move {
      let hw_msg = fut.await?;
      let data = hw_msg.data();
      let battery_reading = message::InputReadingV4::new(
        device_index,
        feature_index,
        InputTypeData::Battery(InputData::new(data[0])),
      );
      debug!("Got battery reading: {}", data[0]);
      Ok(battery_reading)
    }
        .boxed()
  }
}

#[derive(Clone, Debug)]
pub struct VibrateGroup {
  pub work_mode: u8,
  pub motor_pos: u8,
  pub time_100ms: u16,
  pub freq: u16,
  pub strength: u8,
}

fn encode_vibrate_group(g: &VibrateGroup) -> [u8; 8] {
  let mode_pos: u8 = ((g.motor_pos & 0x0F) << 4) | (g.work_mode & 0x0F);
  [
    mode_pos,
    0x00,
    0x05,
    (g.time_100ms >> 8) as u8,
    (g.time_100ms & 0xFF) as u8,
    (g.freq >> 8) as u8,
    (g.freq & 0xFF) as u8,
    g.strength,
  ]
}

fn build_vibrate_data(random: &[u8], groups: &[VibrateGroup]) -> Result<Vec<u8>, String> {
  if random.len() != 16 {
    return Err("random len not valid".into());
  }
  let mut data: Vec<u8> = Vec::new();
  for g in groups {
    data.extend_from_slice(&encode_vibrate_group(g));
  }
  let data_len: u16 = (data.len() as u16).saturating_add(8);
  let type_byte: u8 = 0xB1;
  let cmd_byte: u8 = 0x03;
  let mut hasher = Md5::new();
  hasher.update([type_byte]);
  hasher.update([cmd_byte]);
  hasher.update([(data_len >> 8) as u8]);
  hasher.update([(data_len & 0xFF) as u8]);
  hasher.update(&data);
  hasher.update(&SECRET);
  hasher.update(random);
  let digest = hasher.finalize();
  let md58 = &digest.as_slice()[..8];
  let mut payload = data.clone();
  payload.extend_from_slice(md58);
  Ok(payload)
}

// Represents a complete decoded response frame.
pub struct ResponsePacket {
  pub cmd_type: u8,
  pub cmd: u8,
  pub length: u16,
  pub data: Vec<u8>,
  pub response_code: Option<u16>,
}

// Provides static helpers for encoding and decoding BLE frames.
pub struct FrameCodec;

impl FrameCodec {
  pub fn build_frame(cmd_type: u8, cmd: u8, data: &[u8], counter: u8) -> Vec<u8> {
    let stx = 0x02u8;
    let flag = [0xA5u8, 0x5A, 0x55, 0xAA, 0xF0];
    let len = data.len() as u16;
    let etx = 0x03u8;

    let mut crc_data = vec![cmd_type, cmd, (len >> 8) as u8, (len & 0xFF) as u8];
    crc_data.extend_from_slice(data);
    let crc16 = Self::crc16_modbus(&crc_data);

    let mut frame = Vec::new();
    frame.push(stx);
    frame.extend_from_slice(&flag);
    frame.push(counter);
    frame.push(cmd_type);
    frame.push(cmd);
    frame.push((len >> 8) as u8);
    frame.push((len & 0xFF) as u8);
    frame.extend_from_slice(data);
    frame.push(etx);
    frame.push((crc16 >> 8) as u8);
    frame.push((crc16 & 0xFF) as u8);
    frame
  }

  pub fn crc16_modbus(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &b in data {
      crc ^= b as u16;
      for _ in 0..8 {
        crc = if crc & 1 != 0 {
          (crc >> 1) ^ 0xA001
        } else {
          crc >> 1
        };
      }
    }
    crc
  }

  pub fn extract_random(frame: &[u8]) -> Option<[u8; 16]> {
    let data_len = ((frame[9] as usize) << 8) | (frame[10] as usize);
    let data_start = 11;
    let data_end = data_start + data_len;
    if frame.len() < data_end || data_len < 18 {
      return None;
    }
    let rand_start = data_start + 2;
    let rand_end = rand_start + 16;
    if rand_end <= data_end {
      let mut r = [0u8; 16];
      r.copy_from_slice(&frame[rand_start..rand_end]);
      Some(r)
    } else {
      None
    }
  }

  pub fn parse_response_frame(frame: &[u8]) -> Option<ResponsePacket> {
    if frame.len() < 15 {
      return None;
    }
    let cmd_type = frame[7];
    let cmd = frame[8];
    let len = ((frame[9] as u16) << 8) | frame[10] as u16;
    let data_start = 11;
    let data_end = data_start + len as usize;
    if frame.len() < data_end + 3 {
      return None;
    }
    let data = frame[data_start..data_end].to_vec();
    let response_code = if data.len() >= 2 {
      Some(((data[0] as u16) << 8) | data[1] as u16)
    } else {
      None
    };
    Some(ResponsePacket {
      cmd_type,
      cmd,
      length: len,
      data,
      response_code,
    })
  }
}

pub struct FrameCollector {
  buffer: Vec<u8>,
}

impl FrameCollector {
  pub fn new() -> Self {
    Self { buffer: Vec::new() }
  }

  pub fn push_bytes(&mut self, data: &[u8]) -> Vec<Vec<u8>> {
    self.buffer.extend_from_slice(data);
    let mut frames = Vec::new();

    loop {
      let start_pos = match self
        .buffer
        .windows(6)
        .position(|w| w == [0x02, 0xA5, 0x5A, 0x55, 0xAA, 0xF0])
      {
        Some(pos) => pos,
        None => {
          self.buffer.clear();
          break;
        }
      };
      // Discard the garbage bytes before the frame header.
      if start_pos > 0 {
        self.buffer.drain(0..start_pos);
      }
      // If the buffer length is shorter than the minimum frame length (header + flags + len + etc.)
      if self.buffer.len() < 11 {
        break;
      }

      let len_hi = *self.buffer.get(9).unwrap_or(&0) as usize;
      let len_lo = *self.buffer.get(10).unwrap_or(&0) as usize;
      let data_len = (len_hi << 8) | len_lo;
      // Calculate the complete frame length: STX + FLAG + CNT + TYPE/CMD/LEN(4B) + DATA + ETX + CRC16
      let frame_len = 1 + 5 + 1 + 1 + 1 + 2 + data_len + 1 + 2;

      if self.buffer.len() < frame_len {
        break;
      }
      let etx_pos = 1 + 5 + 1 + 1 + 1 + 2 + data_len;
      if self.buffer.get(etx_pos) != Some(&0x03) {
        // If it's not a valid frame tail, discard the first byte and continue.
        self.buffer.drain(0..1);
        continue;
      }
      // Full frame
      let frame: Vec<u8> = self.buffer.drain(0..frame_len).collect();
      trace!("FrameCollector: assembled frame (len={})", frame.len());
      frames.push(frame);
      // If the remaining data is too short to construct a new frame, exit and wait.
      if self.buffer.len() < 11 {
        break;
      }
    }
    frames
  }
}
