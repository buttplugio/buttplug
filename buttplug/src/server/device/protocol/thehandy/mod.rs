// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use self::handyplug::Ping;

use super::fleshlight_launch_helper;
use crate::server::device::configuration::ProtocolDeviceAttributes;
use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{self, ButtplugDeviceMessage, Endpoint},
  },
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
use prost::Message;
use std::sync::{
  atomic::{AtomicU8, Ordering},
  Arc,
};

mod protocomm {
  include!("./protocomm.rs");
}

mod handyplug {
  include!("./handyplug.rs");
}

generic_protocol_initializer_setup!(TheHandy, "thehandy");

#[derive(Default)]
pub struct TheHandyInitializer {}

#[async_trait]
impl ProtocolInitializer for TheHandyInitializer {
  async fn initialize(
    &mut self,
    _hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
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
pub struct TheHandy {
  // The generic command manager would normally handle this storage, but the only reason we're
  // retaining tracking information is to build our fucking timing calculation for the fleshlight
  // command backport. I am so mad right now.
  previous_position: Arc<AtomicU8>,
}

impl ProtocolHandler for TheHandy {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    let ping_payload = handyplug::Payload {
      messages: vec![handyplug::Message {
        message: Some(handyplug::message::Message::Ping(Ping { id: 999 })),
      }],
    };
    let mut ping_buf = vec![];
    ping_payload
      .encode(&mut ping_buf)
      .expect("Infallible encode.");

    super::ProtocolKeepaliveStrategy::RepeatPacketStrategy(HardwareWriteCmd::new(
      Endpoint::Tx,
      ping_buf,
      true,
    ))
  }

  fn handle_fleshlight_launch_fw12_cmd(
    &self,
    message: message::FleshlightLaunchFW12Cmd,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    // Oh good. ScriptPlayer hasn't updated to LinearCmd yet so now I have to
    // work backward from fleshlight to my own Linear format that Handy uses.
    //
    // Building this library was a mistake.
    let goal_position = message.position() as f64 / 100f64;
    let previous_position = self.previous_position.load(Ordering::SeqCst) as f64 / 100f64;
    self
      .previous_position
      .store(message.position(), Ordering::SeqCst);
    let distance = (goal_position - previous_position).abs();
    let duration =
      fleshlight_launch_helper::calculate_duration(distance, message.speed() as f64 / 99f64) as u32;
    self.handle_linear_cmd(message::LinearCmd::new(
      message.device_index(),
      vec![message::VectorSubcommand::new(0, duration, goal_position)],
    ))
  }

  fn handle_linear_cmd(
    &self,
    message: message::LinearCmd,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    // What is "How not to implement a command structure for your device that
    // does one thing", Alex?

    // First make sure we only have one vector.
    //
    // TODO Use the command manager to check this.
    if message.vectors().len() != 1 {
      return Err(ButtplugDeviceError::DeviceFeatureCountMismatch(
        1,
        message.vectors().len() as u32,
      ));
    }

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
        duration: message.vectors()[0].duration(),
        position: message.vectors()[0].position(),
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
    Ok(vec![
      HardwareWriteCmd::new(Endpoint::Tx, linear_buf, true).into()
    ])
  }
}
