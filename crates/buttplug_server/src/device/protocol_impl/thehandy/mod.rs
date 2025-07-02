// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use self::handyplug::Ping;

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
  protocol::{
    generic_protocol_initializer_setup,
    ProtocolHandler,
    ProtocolIdentifier,
    ProtocolInitializer, ProtocolKeepaliveStrategy,
  },
};
use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use buttplug_server_device_config::{
  DeviceDefinition,
  ProtocolCommunicationSpecifier,
  UserDeviceIdentifier,
};
use prost::Message;
use std::sync::Arc;
use uuid::{uuid, Uuid};

mod protocomm {
  include!("./protocomm.rs");
}

mod handyplug {
  include!("./handyplug.rs");
}

const THEHANDY_PROTOCOL_UUID: Uuid = uuid!("e7c3ba93-ddbf-4f38-a960-30a332739d02");
generic_protocol_initializer_setup!(TheHandy, "thehandy");

#[derive(Default)]
pub struct TheHandyInitializer {}

#[async_trait]
impl ProtocolInitializer for TheHandyInitializer {
  async fn initialize(
    &mut self,
    _hardware: Arc<Hardware>,
    _: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    // Ok, somehow this whole function has been basically a no-op. The read/write lines never had an
    // await on them, so they were never run. But only now, in Rust 1.75/Buttplug 7.1.15, have we
    // gotten a complaint from the compiler. Going to comment this out for now and see what happens.
    // If we don't get any complaints, I'm going to have to rewrite all of my snark here. :(

    // Ok, here we go. This is an overly-complex nightmare but apparently "protocomm makes the
    // firmware easier".
    //
    // This code is mostly my translation of the Handy Python POC. It leaves out a lot of stuff
    // that doesn't seem needed (ping messages, the whole RequestServerInfo flow, etc...) If they
    // ever change anything, I quit.
    //
    // If you are a sex toy manufacturer reading this code: Please, talk to me before implementing
    // your protocol. Buttplug is not made to be a hardware/firmware protocol, and you will regret
    // trying to make it such.

    // First we need to set up a session with The Handy. This will require sending the "security
    // initializer" to basically say we're sending plaintext. Due to pb3 making everything
    // optional, we have some Option<T> wrappers here.

    // let session_req = protocomm::SessionData {
    //   sec_ver: protocomm::SecSchemeVersion::SecScheme0 as i32,
    //   proto: Some(protocomm::session_data::Proto::Sec0(
    //     protocomm::Sec0Payload {
    //       msg: protocomm::Sec0MsgType::S0SessionCommand as i32,
    //       payload: Some(protocomm::sec0_payload::Payload::Sc(
    //         protocomm::S0SessionCmd {},
    //       )),
    //     },
    //   )),
    // };

    // We need to shove this at what we're calling the "firmware" endpoint but is actually the
    // "prov-session" characteristic. These names are stored in characteristic descriptors, which
    // isn't super common on sex toys (with exceptions for things that have a lot of sensors, like
    // the Lelo F1s).
    //
    // I don't have to do characteristic descriptor lookups for the other 140+ pieces of hardware
    // this library supports so I'm damn well not doing it now. YOLO'ing hardcoded values from the
    // device config.
    //
    // If they ever change this, I quit (or will just update the device config).

    // hardware.write_value(&HardwareWriteCmd::new(Endpoint::Firmware, sec_buf, false));
    // hardware.read_value(&HardwareReadCmd::new(Endpoint::Firmware, 100, 500));

    // At this point, the "handyplug" protocol does actually have both RequestServerInfo and Ping
    // messages that it can use. However, having removed these and still tried to run the system,
    // it seems fine. I've omitted those for the moment, and will readd the complexity once it
    // does not seem needless.
    //
    // We have no device name updates here, so just return a device.
    Ok(Arc::new(TheHandy::default()))
  }
}

#[derive(Default)]
pub struct TheHandy {}

impl ProtocolHandler for TheHandy {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    let ping_payload = handyplug::Payload {
      messages: vec![handyplug::Message {
        message: Some(handyplug::message::Message::Ping(Ping { id: 999 })),
      }],
    };
    let mut ping_buf = vec![];
    ping_payload
      .encode(&mut ping_buf)
      .expect("Infallible encode.");

    ProtocolKeepaliveStrategy::HardwareRequiredRepeatPacketStrategy(HardwareWriteCmd::new(
      &[THEHANDY_PROTOCOL_UUID],
      Endpoint::Tx,
      ping_buf,
      true,
    ))
  }

  fn handle_position_with_duration_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    position: u32,
    duration: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    // What is "How not to implement a command structure for your device that does one thing", Alex?

    let linear = handyplug::LinearCmd {
      // You know when message IDs are important? When you have a protocol that handles multiple
      // asynchronous commands. You know what doesn't handle multiple asynchronous commands? The
      // handyplug protocol.
      //
      // Do you know where you'd pack those? In the top level container, as they should then be
      // separate from the message context, in order to allow multiple sorters. Do you know what
      // doesn't need multiple sorters? The handyplug protocol.
      //
      // Please do not cargo cult protocols.
      id: 2,
      // You know when multiple device indicies are important? WHEN YOU HAVE MULTIPLE DEVICE
      // CONNECTI... oh fuck it. I am so tired. I am going to bed.
      device_index: 0,
      // AND I'M BACK AND WELL RESTED. You know when multiple axes are important? When you have to
      // support arbitrary devices with multiple axes. You know what device doesn't have multiple
      // axes?
      //
      // Guess.
      //
      // I'll wait.
      //
      // The handy. It's the handy.
      vectors: vec![handyplug::linear_cmd::Vector {
        index: 0,
        position: position as f64 / 100f64,
        duration,
      }],
    };
    let linear_payload = handyplug::Payload {
      messages: vec![handyplug::Message {
        message: Some(handyplug::message::Message::LinearCmd(linear)),
      }],
    };
    let mut linear_buf = vec![];
    linear_payload
      .encode(&mut linear_buf)
      .expect("Infallible encode.");
    Ok(vec![HardwareWriteCmd::new(
      &[THEHANDY_PROTOCOL_UUID],
      Endpoint::Tx,
      linear_buf,
      true,
    )
    .into()])
  }
}
