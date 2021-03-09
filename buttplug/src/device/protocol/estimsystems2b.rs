use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolCommandHandler};
use crate::{
  core::errors::{ButtplugDeviceError, ButtplugError},
  core::messages::{self, ButtplugDeviceCommandMessageUnion, DeviceMessageAttributesMap},
  core::ButtplugResult,
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    ButtplugDeviceEvent,
    DeviceImpl,
    DeviceSubscribeCmd,
    DeviceWriteCmd,
    Endpoint,
  },
};
use futures::{future::BoxFuture, FutureExt};
use futures_timer::Delay;
use std::{str::FromStr, sync::Arc, time::Duration};
use tokio::sync::Mutex;

/// Contains data from the response from the 2B.
///
/// Note that only the controls that are addressable via Buttplug are represented here.
#[derive(PartialEq, Debug)]
struct EstimSystems2BStatus {
  channel_a: u8,
  channel_b: u8,
  adjust_c: u8,
  adjust_d: u8,
  battery_level: u16,
  firmware_version: String,
}

impl EstimSystems2BStatus {
  pub fn parse_value<T: FromStr>(data: &str) -> ButtplugResult<T> {
    match data.parse::<T>() {
      Ok(v) => Ok(v),
      Err(_) => Err(
        ButtplugDeviceError::ProtocolSpecificError(
          "Estim Systems 2B".to_owned(),
          format!("Failed to parse number from {}", data),
        )
        .into(),
      ),
    }
  }

  pub fn from(data: &str) -> ButtplugResult<EstimSystems2BStatus> {
    let data = data
      .split('\n')
      .collect::<Vec<&str>>()
      .get(0)
      .unwrap_or(&"")
      .to_owned();

    const ESTIMSYSTEMS2B_RESPONSE_PARTS: usize = 9;
    let data: Vec<&str> = data.split(':').collect();
    if data.len() != ESTIMSYSTEMS2B_RESPONSE_PARTS {
      return Err(
        ButtplugDeviceError::ProtocolSpecificError(
          "Estim Systems 2B".to_owned(),
          format!(
            "Response from powerbox had unexpected number of parts (got {}, expected {})",
            data.len(),
            ESTIMSYSTEMS2B_RESPONSE_PARTS
          ),
        )
        .into(),
      );
    }

    let channel_a: u8 = EstimSystems2BStatus::parse_value::<u8>(data[1])? / 2;
    let channel_b: u8 = EstimSystems2BStatus::parse_value::<u8>(data[2])? / 2;
    let adjust_c: u8 = EstimSystems2BStatus::parse_value::<u8>(data[3])? / 2;
    let adjust_d: u8 = EstimSystems2BStatus::parse_value::<u8>(data[4])? / 2;
    let battery_level: u16 = EstimSystems2BStatus::parse_value(data[0])?;
    let firmware_version = data[8].to_owned();

    Ok(EstimSystems2BStatus {
      channel_a,
      channel_b,
      adjust_c,
      adjust_d,
      battery_level,
      firmware_version,
    })
  }
}

#[derive(ButtplugProtocolProperties)]
pub struct EstimSystems2B {
  name: String,
  message_attributes: DeviceMessageAttributesMap,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl EstimSystems2B {
  fn retrieve_status(device: Arc<DeviceImpl>) -> ButtplugDeviceResultFuture {
    Box::pin(async move {
      let mut stream = device.event_stream();
      let mut received_data = String::new();
      let mut retry_counter: i32 = 0;
      loop {
        select! {
          event = stream.recv().fuse() => {
              match event {
                  Ok(ButtplugDeviceEvent::Notification(_, _, data)) => {
                      if let Ok(data) = String::from_utf8(data.to_vec()) {
                          received_data.push_str(&data);
                          match EstimSystems2BStatus::from(&received_data) {
                              Ok(status) => {
                                  info!("Powerbox status: {:?}", status);
                                  return Ok(messages::Ok::default().into());
                              },
                              Err(_) => continue,
                          }
                      } else {
                          return Err(
                              ButtplugDeviceError::ProtocolSpecificError(
                                  "Estim Systems 2B".to_owned(),
                                  "Failed to receive valid data from powerbox".to_owned()).into());
                      }
                  },
                  _ => {
                      info!("Unhandled event: {:?}", event);
                  },
              }
          },
          _ = Delay::new(Duration::from_millis(100)).fuse() => {
              retry_counter += 1;
              if retry_counter > 5 {
                  warn!("Failed to parse status in a sensible time");
                  return Err(
                      ButtplugDeviceError::ProtocolSpecificError(
                          "Estim Systems 2B".to_owned(),
                          "Timed out while getting status from powerbox".to_owned()).into());
              }
          }
        }
      }
    })
  }
}

impl ButtplugProtocol for EstimSystems2B {
  fn new_protocol(
    name: &str,
    message_attributes: DeviceMessageAttributesMap,
  ) -> Box<dyn ButtplugProtocol>
  where
    Self: Sized,
  {
    let manager = GenericCommandManager::new(&message_attributes);

    Box::new(Self {
      name: name.to_owned(),
      message_attributes,
      stop_commands: manager.get_stop_commands(),
      manager: Arc::new(Mutex::new(manager)),
    })
  }

  fn initialize(
    device_impl: Arc<DeviceImpl>,
  ) -> BoxFuture<'static, Result<Option<String>, ButtplugError>> {
    Box::pin(async move {
      device_impl
        .subscribe(DeviceSubscribeCmd::new(Endpoint::Rx))
        .await?;
      Ok(None)
    })
  }
}

impl ButtplugProtocolCommandHandler for EstimSystems2B {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<DeviceImpl>,
    message: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    let manager = self.manager.clone();

    Box::pin(async move {
      let result = manager.lock().await.update_vibration(&message, false)?;
      let mut fut_vec = vec![];

      if let Some(cmds) = result {
        for (i, cmd) in cmds.iter().enumerate() {
          if let Some(val) = cmd {
            let serial_cmd = match i {
              0 => format!("A{}\n\r", val),
              1 => format!("B{}\n\r", val),
              2 => format!("C{}\n\r", val),
              3 => format!("D{}\n\r", val),
              _ => {
                return Err(
                  ButtplugDeviceError::DeviceCommunicationError(format!(
                    "index {} is not expected",
                    i
                  ))
                  .into(),
                )
              }
            };
            info!("2B command: {}", serial_cmd);
            fut_vec.push(device.write_value(DeviceWriteCmd::new(
              Endpoint::Tx,
              serial_cmd.as_bytes().to_vec(),
              false,
            )));
          }
        }
      } else {
        info!("No updates in packet for Estim Systems 2B protocol");
      }

      for fut in fut_vec {
        fut.await?;
        EstimSystems2B::retrieve_status(device.clone()).await?;
      }

      Ok(messages::Ok::default().into())
    })
  }
}

#[cfg(all(test, feature = "server"))]
mod test {
  use super::EstimSystems2BStatus;

  #[test]
  pub fn test_status_decode() {
    assert_eq!(
      EstimSystems2BStatus::from("502:40:160:80:120:0:L:0:2.105\n").unwrap(),
      EstimSystems2BStatus {
        channel_a: 20,
        channel_b: 80,
        adjust_c: 40,
        adjust_d: 60,
        battery_level: 502,
        firmware_version: "2.105".to_owned(),
      }
    );
  }
}
