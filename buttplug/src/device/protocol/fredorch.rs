use super::{
  fleshlight_launch_helper::get_speed,
  ButtplugDeviceResultFuture,
  ButtplugProtocol,
  ButtplugProtocolCommandHandler,
};
use crate::{
  core::errors::ButtplugDeviceError,
  core::messages::{
    self,
    ButtplugDeviceCommandMessageUnion,
    ButtplugDeviceMessage,
    DeviceMessageAttributesMap,
    FleshlightLaunchFW12Cmd,
  },
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    ButtplugDeviceEvent,
    DeviceImpl,
    DeviceProtocolConfiguration,
    DeviceSubscribeCmd,
    DeviceWriteCmd,
    Endpoint,
  },
};
use futures::FutureExt;
use futures_timer::Delay;
use std::{
  sync::{
    atomic::{AtomicU8, Ordering::SeqCst},
    Arc,
  },
  time::Duration,
};
use tokio::sync::Mutex;

const FREDORCH_TIMEOUT_MS: u64 = 500;

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

#[derive(ButtplugProtocolProperties)]
pub struct Fredorch {
  name: String,
  message_attributes: DeviceMessageAttributesMap,
  _manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
  previous_position: Arc<AtomicU8>,
}

impl Fredorch {
  fn new(name: &str, message_attributes: DeviceMessageAttributesMap) -> Self {
    let manager = GenericCommandManager::new(&message_attributes);

    Self {
      name: name.to_owned(),
      message_attributes,
      stop_commands: manager.get_stop_commands(),
      _manager: Arc::new(Mutex::new(manager)),
      previous_position: Arc::new(AtomicU8::new(0)),
    }
  }
}

impl ButtplugProtocol for Fredorch {
  fn try_create(
    device_impl: Arc<DeviceImpl>,
    config: DeviceProtocolConfiguration,
  ) -> futures::future::BoxFuture<
    'static,
    Result<Box<dyn ButtplugProtocol>, crate::core::errors::ButtplugError>,
  > {
    Box::pin(async move {
      let mut event_receiver = device_impl.event_stream();
      device_impl
        .subscribe(DeviceSubscribeCmd::new(Endpoint::Rx))
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
            0x01, 0x10, 0x00, 0x6b, 0x00, 0x05, 0x0a, 0x00, 0x05, 0x00, 0x05, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x01,
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
          if let Ok(ButtplugDeviceEvent::Notification(_, _, n)) = event {
            debug!("Fredorch: wake up - received {:?}", n);
          } else {
            return Err(
              ButtplugDeviceError::ProtocolSpecificError(
                "Fredorch".to_owned(),
                "Fredorch Device disconnected while initialising.".to_owned(),
              )
              .into(),
            );
          }
        }
        _ = Delay::new(Duration::from_millis(FREDORCH_TIMEOUT_MS)).fuse() => {
          // Or not?
        }
      }

      for mut data in init {
        let crc = crc16(&data.1);
        data.1.push(crc[0]);
        data.1.push(crc[1]);
        debug!("Fredorch: {} - sent {:?}", data.0, data.1);
        device_impl
          .write_value(DeviceWriteCmd::new(Endpoint::Tx, data.1.clone(), false))
          .await?;

        select! {
          event = event_receiver.recv().fuse() => {
            if let Ok(ButtplugDeviceEvent::Notification(_, _, n)) = event {
              debug!("Fredorch: {} - received {:?}", data.0, n);
            } else {
              return Err(
                ButtplugDeviceError::ProtocolSpecificError(
                  "Fredorch".to_owned(),
                  "Fredorch Device disconnected while initialising.".to_owned(),
                )
                .into(),
              );
            }
          }
          _ = Delay::new(Duration::from_millis(FREDORCH_TIMEOUT_MS)).fuse() => {
            return Err(
                ButtplugDeviceError::ProtocolSpecificError(
                  "Fredorch".to_owned(),
                  "Fredorch Device timed out while initialising.".to_owned(),
                )
                .into(),
              );
          }
        }
      }

      let (name, attrs) =
        crate::device::protocol::get_protocol_features(device_impl, None, config)?;
      Ok(Box::new(Self::new(&name, attrs)) as Box<dyn ButtplugProtocol>)
    })
  }
}

impl ButtplugProtocolCommandHandler for Fredorch {
  fn handle_linear_cmd(
    &self,
    device: Arc<DeviceImpl>,
    message: messages::LinearCmd,
  ) -> ButtplugDeviceResultFuture {
    let v = message.vectors()[0].clone();
    // In the protocol, we know max speed is 99, so convert here. We have to
    // use AtomicU8 because there's no AtomicF64 yet.
    let previous_position = self.previous_position.load(SeqCst);
    let distance = (previous_position as f64 - (v.position * 99f64)).abs() / 99f64;
    let fl_cmd = FleshlightLaunchFW12Cmd::new(
      message.device_index(),
      (v.position * 99f64) as u8,
      (get_speed(distance, v.duration) * 99f64) as u8,
    );
    self.handle_fleshlight_launch_fw12_cmd(device, fl_cmd)
  }

  fn handle_fleshlight_launch_fw12_cmd(
    &self,
    device: Arc<DeviceImpl>,
    message: messages::FleshlightLaunchFW12Cmd,
  ) -> ButtplugDeviceResultFuture {
    let previous_position = self.previous_position.clone();
    let position = ((message.position() as f64 / 99.0) * 150.0) as u8;
    let speed = ((message.speed() as f64 / 99.0) * 15.0) as u8;
    debug!("Fredorch: move to {} at speed {}", position, speed);
    let mut data: Vec<u8> = vec![
      0x01, 0x10, 0x00, 0x6B, 0x00, 0x05, 0x0a, 0x00, speed, 0x00, speed, 0x00, position, 0x00,
      position, 0x00, 0x01,
    ];
    let crc = crc16(&data);
    data.push(crc[0]);
    data.push(crc[1]);
    let msg = DeviceWriteCmd::new(Endpoint::Tx, data, false);
    let fut = device.write_value(msg);
    Box::pin(async move {
      previous_position.store(position, SeqCst);
      fut.await?;
      Ok(messages::Ok::default().into())
    })
  }
}

#[cfg(all(test, feature = "server"))]
mod test {
  use crate::{
    core::messages::{FleshlightLaunchFW12Cmd, LinearCmd, VectorSubcommand},
    device::{DeviceImplCommand, DeviceWriteCmd, Endpoint},
    server::comm_managers::test::{
      check_test_recv_empty,
      check_test_recv_value,
      new_bluetoothle_test_device,
    },
    util::async_manager,
  };

  #[test]
  #[ignore]
  pub fn test_fredorch_fleshlight_fw12cmd() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("YXlinksSPP")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .get_endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");

      // Initialisation
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x01, 0x06, 0x00, 0x64, 0x00, 0x01, 0x09, 0xd5],
          false,
        )),
      );
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x01, 0x06, 0x00, 0x69, 0x00, 0x00, 0x59, 0xd6],
          false,
        )),
      );
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![
            0x01, 0x10, 0x00, 0x6b, 0x00, 0x05, 0x0a, 0x00, 0x05, 0x00, 0x05, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x01, 0xc0, 0xc3,
          ],
          false,
        )),
      );
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x01, 0x06, 0x00, 0x69, 0x00, 0x01, 0x98, 0x16],
          false,
        )),
      );
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x01, 0x06, 0x00, 0x6a, 0x00, 0x01, 0x68, 0x16],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver));

      // Movement
      device
        .parse_message(FleshlightLaunchFW12Cmd::new(0, 50, 50).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![
            0x01, 0x10, 0x00, 0x6b, 0x00, 0x05, 0x0a, 0x00, 0x07, 0x00, 0x07, 0x00, 0x4b, 0x00,
            0x4b, 0x00, 0x01, 0x2e, 0x7b,
          ],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver));

      device
        .parse_message(FleshlightLaunchFW12Cmd::new(0, 99, 99).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![
            0x01, 0x10, 0x00, 0x6b, 0x00, 0x05, 0x0a, 0x00, 0x0f, 0x00, 0x0f, 0x00, 0x96, 0x00,
            0x96, 0x00, 0x01, 0xbc, 0x52,
          ],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver));
    });
  }

  #[test]
  #[ignore]
  pub fn test_fredorch_linearcmd() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("YXlinksSPP")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .get_endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");

      // Initialisation
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x01, 0x06, 0x00, 0x64, 0x00, 0x01, 0x09, 0xd5],
          false,
        )),
      );
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x01, 0x06, 0x00, 0x69, 0x00, 0x00, 0x59, 0xd6],
          false,
        )),
      );
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![
            0x01, 0x10, 0x00, 0x6b, 0x00, 0x05, 0x0a, 0x00, 0x05, 0x00, 0x05, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x01, 0xc0, 0xc3,
          ],
          false,
        )),
      );
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x01, 0x06, 0x00, 0x69, 0x00, 0x01, 0x98, 0x16],
          false,
        )),
      );
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x01, 0x06, 0x00, 0x6a, 0x00, 0x01, 0x68, 0x16],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver));

      // Movement
      device
        .parse_message(LinearCmd::new(0, vec![VectorSubcommand::new(0, 200, 0.51)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![
            0x01, 0x10, 0x00, 0x6b, 0x00, 0x05, 0x0a, 0x00, 0x07, 0x00, 0x07, 0x00, 0x4b, 0x00,
            0x4b, 0x00, 0x01, 0x2e, 0x7b,
          ],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver));

      device
        .parse_message(LinearCmd::new(0, vec![VectorSubcommand::new(0, 50, 1.0)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![
            0x01, 0x10, 0x00, 0x6b, 0x00, 0x05, 0x0a, 0x00, 0x0f, 0x00, 0x0f, 0x00, 0x96, 0x00,
            0x96, 0x00, 0x01, 0xbc, 0x52,
          ],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver));
    });
  }
}
