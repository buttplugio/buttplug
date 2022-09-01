// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{ActuatorType, Endpoint},
  },
  server::device::{
    configuration::ProtocolAttributesType,
    hardware::{
      Hardware,
      HardwareCommand,
      HardwareEvent,
      HardwareSubscribeCmd,
      HardwareUnsubscribeCmd,
      HardwareWriteCmd,
    },
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
use crate::server::device::configuration::ProtocolDeviceAttributes;

generic_protocol_initializer_setup!(LeloF1sV2, "lelo-f1sv2");

#[derive(Default)]
pub struct LeloF1sV2Initializer {}

#[async_trait]
impl ProtocolInitializer for LeloF1sV2Initializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    // The Lelo F1s V2 has a very specific pairing flow:
    // * First the device is turned on in BLE mode (long press)
    // * Then the security endpoint (Whitelist) needs to be read (which we can do via subscribe)
    // * If it returns 0x00,00,00,00,00,00,00,00 the connection isn't not authorised
    // * To authorize, the password must be writen to the characteristic.
    // * If the password is unknown (buttplug lacks a storage mechanism right now), the power button
    //   must be pressed to send the password
    // * The password must not be sent whilst subscribed to the endpoint
    // * Once the password has been sent, the endpoint can be read for status again
    // * If it returns 0x00,00,00,00,00,00,00,00 the connection is authorised
    let mut event_receiver = hardware.event_stream();
    hardware
      .subscribe(&HardwareSubscribeCmd::new(Endpoint::Whitelist))
      .await?;
    let noauth: Vec<u8> = vec![0; 8];
    let authed: Vec<u8> = vec![1, 0, 0, 0, 0, 0, 0, 0];

    loop {
      let event = event_receiver.recv().await;
      if let Ok(HardwareEvent::Notification(_, _, n)) = event {
        if n.eq(&noauth) {
          info!(
            "Lelo F1s V2 isn't authorised: Tap the device's power button to complete connection."
          )
        } else if n.eq(&authed) {
          debug!("Lelo F1s V2 is authorised!");
          return Ok(Arc::new(LeloF1sV2::default()));
        } else {
          debug!("Lelo F1s V2 gave us a password: {:?}", n);
          // Can't send whilst subscribed
          hardware
            .unsubscribe(&HardwareUnsubscribeCmd::new(Endpoint::Whitelist))
            .await?;
          // Send with response
          hardware
            .write_value(&HardwareWriteCmd::new(Endpoint::Whitelist, n, true))
            .await?;
          // Get back to the loop
          hardware
            .subscribe(&HardwareSubscribeCmd::new(Endpoint::Whitelist))
            .await?;
        }
      } else {
        return Err(ButtplugDeviceError::ProtocolSpecificError(
          "LeloF1sV2".to_owned(),
          "Lelo F1s V2 didn't provided valid security handshake".to_owned(),
        ));
      }
    }
  }
}

#[derive(Default)]
pub struct LeloF1sV2 {}

impl ProtocolHandler for LeloF1sV2 {
  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut cmd_vec = vec![0x1];
    for cmd in cmds.iter() {
      cmd_vec.push(cmd.expect("LeloF1s should always send all values").1 as u8);
    }
    Ok(vec![
      HardwareWriteCmd::new(Endpoint::Tx, cmd_vec, false).into()
    ])
  }
}

// TODO Gonna need to add the ability to set subscribe data in tests before
// writing the Lelo F1S V2 tests.
/*
#[cfg(all(test, feature = "server"))]
mod test {
  use crate::{
    core::messages::{Endpoint, StopDeviceCmd, VibrateCmd, VibrateSubcommand},
    server::device::{
      device::hardware::{HardwareCommand, DeviceWriteCmd},
      hardware::communication::test::{
        check_test_recv_empty,
        check_test_recv_value,
        new_bluetoothle_test_device,
      },
    },
    util::async_manager,
  };

  #[test]
  pub fn test_lelo_f1s_v2_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("F1SV2A")
        .await
        .expect("Test, assuming infallible");
      let whitelist_sender = test_device
          .get_endpoint_sender(&Endpoint::Whitelist)
          .expect("Test, assuming infallible");
      let whitelist_receiver = test_device
          .get_endpoint_receiver(&Endpoint::Whitelist)
          .expect("Test, assuming infallible");

      // Security handshake
      whitelist_sender.send(vec![0;8]);
      whitelist_sender.send(vec![1,2,3,4,5,6,7,8]);
      check_test_recv_value(
        &whitelist_receiver,
        HardwareCommand::Write(DeviceWriteCmd::new(
          Endpoint::Whitelist,
          vec![1,2,3,4,5,6,7,8],
          false,
        )),
      );
      whitelist_sender.send(vec![1,0,0,0,0,0,0,0]);

      let command_receiver = test_device
        .get_endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x01, 0x32, 0x0],
          false,
        )),
      );
      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver));

      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.1),
              VibrateSubcommand::new(1, 0.5),
            ],
          )
          .into(),
        )
        .await
        .expect("Test, assuming infallible");
      // TODO There's probably a more concise way to do this.
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x1, 0xa, 0x32],
          false,
        )),
      );
      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x1, 0x0, 0x0],
          false,
        )),
      );
    });
  }
}
*/
