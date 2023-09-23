// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::server::device::configuration::ProtocolDeviceAttributes;
use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    configuration::ProtocolAttributesType,
    hardware::{Hardware, HardwareCommand, HardwareEvent, HardwareSubscribeCmd, HardwareWriteCmd},
    protocol::{
      generic_protocol_initializer_setup,
      ProtocolHandler,
      ProtocolIdentifier,
      ProtocolInitializer,
    },
    ServerDeviceIdentifier,
  },
  util::{async_manager, sleep},
};
use async_trait::async_trait;
use futures::FutureExt;
use std::{
  sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
  },
  time::Duration,
};

const FREDORCH_COMMAND_TIMEOUT_MS: u64 = 100;

generic_protocol_initializer_setup!(FredorchRotary, "fredorch-rotary");

#[derive(Default)]
pub struct FredorchRotaryInitializer {}

#[async_trait]
impl ProtocolInitializer for FredorchRotaryInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    warn!(
      "FredorchRotary device doesn't provide state feedback. If the device beeps twice, it is powered off and must be reconnected before it can be controlled!"
    );

    let mut event_receiver = hardware.event_stream();
    hardware
      .subscribe(&HardwareSubscribeCmd::new(Endpoint::Rx))
      .await?;

    let init: Vec<(String, Vec<u8>)> = vec![
      (
        "Start the handshake".to_owned(),
        vec![0x55, 0x03, 0x99, 0x9c, 0xaa],
      ),
      (
        "Send the password".to_owned(),
        vec![
          0x55, 0x09, 0x21, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2a, 0xaa,
        ],
      ),
      (
        "Power up the device".to_owned(),
        vec![0x55, 0x03, 0x1f, 0x22, 0xaa],
      ),
      (
        "Stop the device".to_owned(),
        vec![0x55, 0x03, 0x24, 0x27, 0xaa],
      ),
    ];

    for data in init {
      debug!("FredorchRotary: {} - sent {:?}", data.0, data.1);
      hardware
        .write_value(&HardwareWriteCmd::new(Endpoint::Tx, data.1.clone(), false))
        .await?;

      select! {
        event = event_receiver.recv().fuse() => {
          if let Ok(HardwareEvent::Notification(_, _, n)) = event {
            debug!("FredorchRotary: {} - received {:?}", data.0, n);
          } else {
            return Err(
              ButtplugDeviceError::ProtocolSpecificError(
                "FredorchRotary".to_owned(),
                "FredorchRotary Device disconnected while initialising.".to_owned(),
              )
            );
          }
        }
        _ = sleep(Duration::from_millis(FREDORCH_COMMAND_TIMEOUT_MS)).fuse() => {
          // The after the password check, we won't get anything
        }
      }
    }

    Ok(Arc::new(FredorchRotary::new(hardware)))
  }
}

pub struct FredorchRotary {
  current_speed: Arc<AtomicU8>,
  target_speed: Arc<AtomicU8>,
}

async fn speed_update_handler(
  device: Arc<Hardware>,
  current_speed: Arc<AtomicU8>,
  target_speed: Arc<AtomicU8>,
) {
  info!("Entering FredorchRotary Control Loop");

  loop {
    let ts = target_speed.load(Ordering::SeqCst);
    let cs = current_speed.load(Ordering::SeqCst);

    trace!("FredorchRotary: {}c vs {}t", cs, ts);

    if ts != cs {
      let cmd: u8 = if ts == 0 {
        0x24
      } else if ts > cs {
        0x01
      } else {
        0x02
      };
      let update = device
        .write_value(&HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![0x55u8, 0x03, cmd, cmd + 3, 0xaa],
          false,
        ))
        .await;
      if update.is_ok() {
        debug!(
          "FredorchRotary: {}c vs {}t - speed {:?}",
          cs,
          ts,
          if ts == 0 {
            "STOP"
          } else if ts > cs {
            "+1"
          } else {
            "-1"
          }
        );
        current_speed.store(
          u8::max(
            if ts == 0 {
              0
            } else if ts > cs {
              cs + 1
            } else {
              cs - 1
            },
            0,
          ),
          Ordering::SeqCst,
        );
        continue;
      } else {
        info!("FredorchRotary update error: {:?}", update.err());
        break;
      }
    }

    sleep(Duration::from_millis(FREDORCH_COMMAND_TIMEOUT_MS)).await;
  }
  info!("FredorchRotary control loop exiting, most likely due to device disconnection.");
}

impl FredorchRotary {
  fn new(device: Arc<Hardware>) -> Self {
    let current_speed = Arc::new(AtomicU8::new(0));
    let target_speed = Arc::new(AtomicU8::new(0));
    let current_speed_clone = current_speed.clone();
    let target_speed_clone = target_speed.clone();
    async_manager::spawn(async move {
      speed_update_handler(device, current_speed_clone, target_speed_clone).await
    });
    Self {
      current_speed,
      target_speed,
    }
  }
}

impl ProtocolHandler for FredorchRotary {
  fn handle_scalar_oscillate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let speed: u8 = scalar as u8;

    self.target_speed.store(speed, Ordering::SeqCst);
    if speed == 0 {
      self.current_speed.store(speed, Ordering::SeqCst);
      Ok(vec![HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![0x55, 0x03, 0x24, 0x27, 0xaa],
        false,
      )
      .into()])
    } else {
      Ok(vec![])
    }
  }
}
