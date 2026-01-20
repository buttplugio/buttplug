// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::hardware::{HardwareEvent, HardwareSubscribeCmd};
use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
  protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer, ProtocolKeepaliveStrategy},
};
use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use buttplug_server_device_config::{
  ProtocolCommunicationSpecifier,
  ServerDeviceDefinition,
  UserDeviceIdentifier,
};
use prost::Message;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use uuid::{Uuid, uuid};

mod handy_rpc {
  include!("./hdy_rpc.rs");
}

const THEHANDY_V3_PROTOCOL_UUID: Uuid = uuid!("f148e5a6-91fe-4666-944f-2fcec62846af");
const THEHANDY_V3_SUBSCRIBE_UUID: Uuid = uuid!("55392f4a-7dcb-435f-a33d-38ec4c1d7d7e");

pub mod setup {
  use crate::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};

  #[derive(Default)]
  pub struct TheHandyV3IdentifierFactory {}

  impl ProtocolIdentifierFactory for TheHandyV3IdentifierFactory {
    fn identifier(&self) -> &str {
      "thehandy-v3"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::TheHandyV3Identifier::default())
    }
  }
}

#[derive(Default)]
pub struct TheHandyV3Identifier {}

#[async_trait]
impl ProtocolIdentifier for TheHandyV3Identifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
    _: ProtocolCommunicationSpecifier,
  ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    let bits: Vec<&str> = hardware.name().split('_').collect();
    let name = if bits.len() > 2 { bits[1] } else { "unknown" };
    Ok((
      UserDeviceIdentifier::new(hardware.address(), "thehandy-v3", &Some(name.to_owned())),
      Box::new(TheHandyV3Initializer::default()),
    ))
  }
}

#[derive(Default)]
pub struct TheHandyV3Initializer {}

#[async_trait]
impl ProtocolInitializer for TheHandyV3Initializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let mut event_receiver = hardware.event_stream();
    hardware
      .subscribe(&HardwareSubscribeCmd::new(
        THEHANDY_V3_SUBSCRIBE_UUID,
        Endpoint::Rx,
      ))
      .await?;

    let ping_payload = handy_rpc::RpcMessage {
      r#type: handy_rpc::MessageType::Request as i32,
      message: Some(handy_rpc::rpc_message::Message::Request(
        handy_rpc::Request {
          id: 1,
          params: Some(handy_rpc::request::Params::RequestBatteryGet(
            handy_rpc::RequestBatteryGet {},
          )),
        },
      )),
    };
    let mut ping_buf = vec![];
    ping_payload
      .encode(&mut ping_buf)
      .expect("Infallible encode.");

    debug!("Send {:02X?}", ping_buf);
    let msg = HardwareWriteCmd::new(&[THEHANDY_V3_PROTOCOL_UUID], Endpoint::Tx, ping_buf, false);
    hardware.write_value(&msg).await?;

    let event = event_receiver.recv().await;
    if let Ok(HardwareEvent::Notification(_, _, n)) = event {
      debug!(
        "Got {:02X?} {:?}",
        n,
        handy_rpc::RpcMessage::decode(n.as_slice()) // We get battery info!
      );
    }

    let ping_payload = handy_rpc::RpcMessage {
      r#type: handy_rpc::MessageType::Request as i32,
      message: Some(handy_rpc::rpc_message::Message::Request(
        handy_rpc::Request {
          id: 2,
          params: Some(handy_rpc::request::Params::RequestCapabilitiesGet(
            handy_rpc::RequestCapabilitiesGet {},
          )),
        },
      )),
    };
    let mut ping_buf = vec![];
    ping_payload
      .encode(&mut ping_buf)
      .expect("Infallible encode.");

    debug!("Send {:02X?}", ping_buf);
    let msg = HardwareWriteCmd::new(&[THEHANDY_V3_PROTOCOL_UUID], Endpoint::Tx, ping_buf, false);
    hardware.write_value(&msg).await?;

    let event = event_receiver.recv().await;
    if let Ok(HardwareEvent::Notification(_, _, n)) = event {
      debug!(
        "Got {:02X?} {:?}",
        n,
        handy_rpc::RpcMessage::decode(n.as_slice())
      );
      // I'm only getting errors back for this... Which suggests the protobuf is out-of-sync for my firmware?
    }

    Ok(Arc::new(TheHandyV3::default()))
  }
}

#[derive(Default)]
pub struct TheHandyV3 {
  seq: Arc<AtomicU32>,
}

impl ProtocolHandler for TheHandyV3 {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    let ping_payload = handy_rpc::Request {
      id: self.seq.fetch_add(1, Ordering::Relaxed),
      params: Some(handy_rpc::request::Params::RequestCapabilitiesGet(
        handy_rpc::RequestCapabilitiesGet {},
      )),
    };
    let mut ping_buf = vec![];
    ping_payload
      .encode(&mut ping_buf)
      .expect("Infallible encode.");

    ProtocolKeepaliveStrategy::HardwareRequiredRepeatPacketStrategy(HardwareWriteCmd::new(
      &[THEHANDY_V3_PROTOCOL_UUID],
      Endpoint::Tx,
      ping_buf,
      true,
    ))
  }

  fn handle_hw_position_with_duration_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    position: u32,
    duration: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let linear_payload = handy_rpc::RpcMessage {
      r#type: handy_rpc::MessageType::Request as i32,
      message: Some(handy_rpc::rpc_message::Message::Request(
        handy_rpc::Request {
          id: self.seq.fetch_add(1, Ordering::Relaxed),
          params: Some(handy_rpc::request::Params::RequestHdspXpTSet(
            handy_rpc::RequestHdspXpTSet {
              stop_on_target: true,
              t: duration,                  // time in ms
              xp: position as f32 / 100f32, // position 0.0-1.0
            },
          )),
        },
      )),
    };
    let mut linear_buf = vec![];
    linear_payload
      .encode(&mut linear_buf)
      .expect("Infallible encode.");
    Ok(vec![
      HardwareWriteCmd::new(&[feature_id], Endpoint::Tx, linear_buf, true).into(),
    ])
  }

  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let payload = handy_rpc::RpcMessage {
      r#type: handy_rpc::MessageType::Request as i32,
      message: Some(handy_rpc::rpc_message::Message::Request(
        handy_rpc::Request {
          id: self.seq.fetch_add(1, Ordering::Relaxed),
          params: Some(if speed == 0 {
            handy_rpc::request::Params::RequestHvpStop(handy_rpc::RequestHvpStop {})
          } else {
            handy_rpc::request::Params::RequestHvpSet(handy_rpc::RequestHvpSet {
              position: 0.0,                    // does nothing?
              amplitude: speed as f32 / 100f32, // vibe speed 0.0-1.0
              frequency: 100,                   // Not sure how to map this. 0-10000Hz
            })
          }),
        },
      )),
    };
    let mut buf = vec![];
    payload.encode(&mut buf).expect("Infallible encode.");
    Ok(vec![
      HardwareWriteCmd::new(&[feature_id], Endpoint::Tx, buf, true).into(),
    ])
  }
}
