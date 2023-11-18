#[cfg(feature = "wasm")]
use crate::util;
use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  generic_protocol_initializer_setup,
  server::device::{
    configuration::ProtocolDeviceAttributes,
    hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
    protocol::{
      ProtocolAttributesType,
      ProtocolHandler,
      ProtocolIdentifier,
      ProtocolInitializer,
      ServerDeviceIdentifier,
    },
  },
  util::async_manager,
};
use async_trait::async_trait;
use std::{
  sync::{
    atomic::{AtomicBool, AtomicU16, Ordering},
    Arc,
  },
  time::Duration,
};
use tokio::sync::Notify;

/// Send command, sub-command, and data (sub-command's arguments) with u8 integers
/// This returns ACK packet for the command or Error.
async fn send_command_raw(
  device: Arc<Hardware>,
  packet_number: u8,
  command: u8,
  sub_command: u8,
  data: &[u8],
  rumble_r: Option<Rumble>,
  rumble_l: Option<Rumble>,
) -> Result<(), ButtplugDeviceError> {
  let mut buf = [0x0; 0x40];
  // set command
  buf[0] = command;
  // set packet number
  buf[1] = packet_number;

  // rumble
  if let Some(rumble_l) = rumble_l {
    let rumble_left: [u8; 4] = rumble_l.into();
    buf[2..6].copy_from_slice(&rumble_left);
  }
  if let Some(rumble_r) = rumble_r {
    let rumble_right: [u8; 4] = rumble_r.into();
    buf[6..10].copy_from_slice(&rumble_right);
  }

  // set sub command
  buf[10] = sub_command;
  // set data
  buf[11..11 + data.len()].copy_from_slice(data);

  // send command
  device
    .write_value(&HardwareWriteCmd::new(Endpoint::Tx, buf.to_vec(), false))
    .await
}

/// Send sub-command, and data (sub-command's arguments) with u8 integers
/// This returns ACK packet for the command or Error.
///
/// # Notice
/// If you are using non-blocking mode,
/// it is more likely to fail to validate the sub command reply.
async fn send_sub_command_raw(
  device: Arc<Hardware>,
  packet_number: u8,
  sub_command: u8,
  data: &[u8],
) -> Result<(), ButtplugDeviceError> {
  //use input_report_mode::sub_command_mode::AckByte;

  send_command_raw(device, packet_number, 1, sub_command, data, None, None).await
  /*
  // check reply
  if self.valid_reply() {
      std::iter::repeat(())
          .take(Self::ACK_TRY)
          .flat_map(|()| {
              let mut buf = [0u8; 362];
              self.read(&mut buf).ok()?;
              let ack_byte = AckByte::from(buf[13]);

              match ack_byte {
                  AckByte::Ack { .. } => Some(buf),
                  AckByte::Nack => None
              }
          })
          .next()
          .map(SubCommandReply::Checked)
          .ok_or_else(|| JoyConError::SubCommandError(sub_command, Vec::new()))
  } else {
      Ok(SubCommandReply::Unchecked)
  }
  */
}

/// Send sub-command, and data (sub-command's arguments) with `Command` and `SubCommand`
/// This returns ACK packet for the command or Error.
async fn send_sub_command(
  device: Arc<Hardware>,
  packet_number: u8,
  sub_command: u8,
  data: &[u8],
) -> Result<(), ButtplugDeviceError> {
  send_sub_command_raw(device, packet_number, sub_command as u8, data).await
}

/// Rumble data for vibration.
///
/// # Notice
/// Constraints exist.
/// * frequency - 0.0 < freq < 1252.0
/// * amplitude - 0.0 < amp < 1.799.0
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rumble {
  frequency: f32,
  amplitude: f32,
}

impl Rumble {
  pub fn frequency(self) -> f32 {
    self.frequency
  }

  pub fn amplitude(self) -> f32 {
    self.amplitude
  }

  /// Constructor of Rumble.
  /// If arguments not in line with constraints, args will be saturated.
  pub fn new(freq: f32, amp: f32) -> Self {
    let freq = if freq < 0.0 {
      0.0
    } else if freq > 1252.0 {
      1252.0
    } else {
      freq
    };

    let amp = if amp < 0.0 {
      0.0
    } else if amp > 1.799 {
      1.799
    } else {
      amp
    };

    Self {
      frequency: freq,
      amplitude: amp,
    }
  }

  /// The amplitudes over 1.003 are not safe for the integrity of the linear resonant actuators.
  pub fn is_safe(self) -> bool {
    self.amplitude < 1.003
  }

  /// Generates stopper of rumbling.
  pub fn stop() -> Self {
    Self {
      frequency: 0.0,
      amplitude: 0.0,
    }
  }
}

impl Into<[u8; 4]> for Rumble {
  fn into(self) -> [u8; 4] {
    let encoded_hex_freq = f32::round(f32::log2(self.frequency / 10.0) * 32.0) as u8;

    let hf_freq: u16 = (encoded_hex_freq as u16).saturating_sub(0x60) * 4;
    let lf_freq: u8 = encoded_hex_freq.saturating_sub(0x41) + 1;

    let encoded_hex_amp = if self.amplitude > 0.23 {
      f32::round(f32::log2(self.amplitude * 8.7) * 32.0) as u8
    } else if self.amplitude > 0.12 {
      f32::round(f32::log2(self.amplitude * 17.0) * 16.0) as u8
    } else {
      f32::round(((f32::log2(self.amplitude) * 32.0) - 96.0) / (4.0 - 2.0 * self.amplitude)) as u8
    };

    let hf_amp: u16 = {
      let hf_amp: u16 = encoded_hex_amp as u16 * 2;
      if hf_amp > 0x01FC {
        0x01FC
      } else {
        hf_amp
      }
    }; // encoded_hex_amp<<1;
    let lf_amp: u8 = {
      let lf_amp = encoded_hex_amp / 2 + 64;
      if lf_amp > 0x7F {
        0x7F
      } else {
        lf_amp
      }
    }; // (encoded_hex_amp>>1)+0x40;

    let mut buf = [0u8; 4];

    // HF: Byte swapping
    buf[0] = (hf_freq & 0xFF) as u8;
    // buf[1] = (hf_amp + ((hf_freq >> 8) & 0xFF)) as u8; //Add amp + 1st byte of frequency to amplitude byte
    buf[1] = (hf_amp + (hf_freq.wrapping_shr(8) & 0xFF)) as u8; //Add amp + 1st byte of frequency to amplitude byte

    // LF: Byte swapping
    buf[2] = lf_freq.saturating_add(lf_amp.wrapping_shr(8));
    buf[3] = lf_amp;

    buf
  }
}

generic_protocol_initializer_setup!(NintendoJoycon, "nintendo-joycon");

#[derive(Default)]
pub struct NintendoJoyconInitializer {}

#[async_trait]
impl ProtocolInitializer for NintendoJoyconInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    send_sub_command(hardware.clone(), 0, 72, &[0x01])
      .await
      .map_err(|_| {
        ButtplugDeviceError::DeviceConnectionError("Cannot initialize joycon".to_owned())
      })?;
    Ok(Arc::new(NintendoJoycon::new(hardware)))
  }
}

pub struct NintendoJoycon {
  //packet_number: Arc<AtomicU8>,
  speed_val: Arc<AtomicU16>,
  notifier: Arc<Notify>,
  is_stopped: Arc<AtomicBool>,
}

impl NintendoJoycon {
  fn new(hardware: Arc<Hardware>) -> Self {
    let speed_val = Arc::new(AtomicU16::new(0));
    let speed_val_clone = speed_val.clone();
    let notifier = Arc::new(Notify::new());
    #[cfg(not(feature = "wasm"))]
    let notifier_clone = notifier.clone();
    let is_stopped = Arc::new(AtomicBool::new(false));
    let is_stopped_clone = is_stopped.clone();
    async_manager::spawn(async move {
      loop {
        if is_stopped_clone.load(Ordering::Relaxed) {
          return;
        }
        let amp = speed_val_clone.load(Ordering::Relaxed) as f32 / 1000f32;
        let rumble = if amp > 0.001 {
          Rumble::new(200.0f32, amp)
        } else {
          Rumble::stop()
        };

        if let Err(_) =
          send_command_raw(hardware.clone(), 1, 16, 0, &[], Some(rumble), Some(rumble)).await
        {
          error!("Joycon command failed, exiting update loop");
          break;
        }
        #[cfg(not(feature = "wasm"))]
        let _ = tokio::time::timeout(Duration::from_millis(15), notifier_clone.notified()).await;

        // If we're using WASM, we can't use tokio's timeout due to lack of time library in WASM.
        // I'm also too lazy to make this a select. So, this'll do. We can't even access this
        // protocol in a web context yet since there's no WebHID comm manager yet.
        #[cfg(feature = "wasm")]
        util::sleep(Duration::from_millis(15)).await;
      }
    });
    Self {
      //packet_number: Arc::new(AtomicU8::new(0)),
      speed_val,
      notifier,
      is_stopped,
    }
  }
}

impl ProtocolHandler for NintendoJoycon {
  fn handle_scalar_vibrate_cmd(
    &self,
    _: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speed_val.store(scalar as u16, Ordering::Relaxed);
    Ok(vec![])
  }
}

impl Drop for NintendoJoycon {
  fn drop(&mut self) {
    self.is_stopped.store(false, Ordering::Relaxed);
    self.notifier.notify_one();
  }
}
