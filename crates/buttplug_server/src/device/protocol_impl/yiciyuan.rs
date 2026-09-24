// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;
use uuid::{Uuid, uuid};

use futures_util::future::BoxFuture;
use futures_util::{FutureExt, future};

use buttplug_core::errors::ButtplugDeviceError;
use buttplug_core::message::{InputReadingV4, InputType, InputTypeReading, InputValue};
use buttplug_core::util::async_manager;
use buttplug_server_device_config::Endpoint;

use buttplug_server_device_config::{
  ProtocolCommunicationSpecifier,
  ServerDeviceDefinition,
  UserDeviceIdentifier,
};

use crate::device::{
  hardware::{
    Hardware,
    HardwareCommand,
    HardwareEvent,
    HardwareSubscribeCmd,
    HardwareUnsubscribeCmd,
    HardwareWriteCmd,
  },
  protocol::{
    ProtocolHandler,
    ProtocolIdentifier,
    ProtocolInitializer,
    generic_protocol_initializer_setup,
  },
};
use crate::message::checked_output_cmd::CheckedOutputCmdV4;

const YICIYUAN_PROTOCOL_UUID: Uuid = uuid!("d5987116-2fba-4c30-a7aa-ef567a3bf35d");
// C-mode writes share the physical FF41 characteristic with A/B live writes,
// but are independent protocol state. A separate command identity prevents the
// device-task batcher from treating one packet family as a replacement for the other.
const YICIYUAN_C_MODE_COMMAND_UUID: Uuid = uuid!("b5ccbc68-d970-4e91-b0fa-7ebf74efbb91");

// Strength remains unsigned. FJB-03's host motion controller adds the selected
// direction (20 + amplitude for reverse) immediately before the actual write.
const MOTOR_MAX_DEFAULT: u8 = 0x14;
const MOTOR_C_MODES_FJB03: u32 = 7;

// Output feature indices, matching the YAML order for the three motor slots.
const FEATURE_STROKE: u32 = 0;
const FEATURE_B: u32 = 1;
const FEATURE_AXIS_C: u32 = 2;

generic_protocol_initializer_setup!(Yiciyuan, "yiciyuan");

#[derive(Default)]
pub struct YiciyuanInitializer {}

// App 6.8.7: settle after discovery, subscribe FF42, settle again, then
// query device info using the model-specific frame. No motor command here.
async fn initialize_fjb(
  hardware: &Hardware,
  settle: Duration,
  timeout: Duration,
) -> Result<(), ButtplugDeviceError> {
  let initialization = async {
    async_manager::sleep(settle).await;
    hardware
      .subscribe(&HardwareSubscribeCmd::new(
        YICIYUAN_PROTOCOL_UUID,
        Endpoint::RxBLEBattery,
      ))
      .await?;
    info!(
      "{} initialization: notifications subscribed",
      hardware.name()
    );
    async_manager::sleep(settle).await;
    let query = if hardware.name() == "YCY-FJB-03" {
      vec![0x35, 0x10, 0x00, 0x00, 0x00, 0x45]
    } else {
      let mut query = vec![0u8; 16];
      query[0] = 0x35;
      query[1] = 0x10;
      query
    };
    hardware
      .write_value(&HardwareWriteCmd::new(
        &[YICIYUAN_PROTOCOL_UUID],
        Endpoint::Tx,
        query,
        false,
      ))
      .await?;
    info!(
      "{} initialization: device-info query submitted (not an execution acknowledgement)",
      hardware.name()
    );
    Ok(())
  };
  tokio::select! {
    result = initialization => result,
    _ = async_manager::sleep(timeout) => Err(ButtplugDeviceError::DeviceCommunicationError(
      format!("{} notification/info initialization timed out", hardware.name()))),
  }
}

#[async_trait]
impl ProtocolInitializer for YiciyuanInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _def: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    if matches!(hardware.name().as_str(), "YCY-FJB-02" | "YCY-FJB-03") {
      initialize_fjb(&hardware, Duration::from_secs(1), Duration::from_secs(7)).await?;
    }
    Ok(Arc::new(Yiciyuan {
      is_fjb03: hardware.name() == "YCY-FJB-03",
      is_fjb02: hardware.name() == "YCY-FJB-02",
      stroke: AtomicU8::new(0),
      motor_b: AtomicU8::new(0),
      axis_c: AtomicU8::new(0),
    }))
  }
}

/// Per-device state. FJB-01/02 send their three motor slots in one packet.
/// FJB-03 uses the live A/B packet plus a separate fixed-mode C command.
pub struct Yiciyuan {
  is_fjb03: bool,
  is_fjb02: bool,
  stroke: AtomicU8,
  motor_b: AtomicU8,
  axis_c: AtomicU8,
}

impl Yiciyuan {
  fn store(&self, feature_index: u32, value: u32) -> Result<(), ButtplugDeviceError> {
    if self.is_fjb03 && feature_index == FEATURE_AXIS_C {
      let mode = if value == 0 {
        0
      } else {
        ((value.min(100) * MOTOR_C_MODES_FJB03 + 99) / 100) as u8
      };
      self.axis_c.store(mode, Ordering::Relaxed);
      return Ok(());
    }
    let level = ((value.min(100) as u16 * MOTOR_MAX_DEFAULT as u16 + 50) / 100) as u8;
    match feature_index {
      FEATURE_STROKE => self.stroke.store(level, Ordering::Relaxed),
      FEATURE_B => self.motor_b.store(level, Ordering::Relaxed),
      FEATURE_AXIS_C => self.axis_c.store(level, Ordering::Relaxed),
      _ => {
        return Err(ButtplugDeviceError::ProtocolSpecificError(
          "Yiciyuan".to_owned(),
          format!("Unknown feature index {}", feature_index),
        ));
      }
    }
    Ok(())
  }

  fn build_packet(&self) -> Vec<u8> {
    let stroke = self.stroke.load(Ordering::Relaxed);
    let motor_b = self.motor_b.load(Ordering::Relaxed);
    if self.is_fjb03 {
      let body = [0x35u8, 0x12, stroke, motor_b, 0x00];
      let checksum = body.iter().fold(0u16, |sum, byte| sum + *byte as u16) as u8;
      let mut packet = Vec::from(body);
      packet.push(checksum);
      return packet;
    }
    // FJB-01/02: 16-byte motor-state frame with reserved bytes zero-padded.
    // FJB-02 only exposes motor A, which mechanically couples two motions.
    let mut packet = vec![0u8; 16];
    packet[0] = 0x35;
    packet[1] = 0x12;
    packet[2] = stroke;
    packet[3] = motor_b;
    packet[4] = self.axis_c.load(Ordering::Relaxed);
    packet
  }

  fn build_fjb03_c_mode_packet(&self) -> Vec<u8> {
    let mode = self.axis_c.load(Ordering::Relaxed);
    let body = [0x35u8, 0x11, 0x04, mode];
    let checksum = body.iter().fold(0u16, |sum, byte| sum + *byte as u16) as u8;
    let mut packet = Vec::from(body);
    packet.push(checksum);
    packet
  }

  fn write_packet(&self, packet: Vec<u8>) -> HardwareCommand {
    let command_id = if packet.get(1) == Some(&0x11) {
      YICIYUAN_C_MODE_COMMAND_UUID
    } else {
      YICIYUAN_PROTOCOL_UUID
    };
    HardwareWriteCmd::new(&[command_id], Endpoint::Tx, packet, false).into()
  }

  fn handle_axis_cmd(
    &self,
    feature_index: u32,
    value: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.store(feature_index, value)?;
    if self.is_fjb03 && feature_index == FEATURE_AXIS_C {
      let mut commands = vec![self.write_packet(self.build_fjb03_c_mode_packet())];
      if value == 0 {
        // Reapply the current A/B levels, whether this was a C-only stop or
        // part of StopCmd. The C field in that packet is always zero.
        commands.push(self.write_packet(self.build_packet()));
      }
      return Ok(commands);
    }

    let mut commands = vec![self.write_packet(self.build_packet())];
    if self.is_fjb03 && self.axis_c.load(Ordering::Relaxed) > 0 {
      // A live A/B frame may cancel the fixed C mode. Reapply it so moving
      // stroke/suction controls does not silently turn vibration off.
      commands.push(self.write_packet(self.build_fjb03_c_mode_packet()));
    }
    Ok(commands)
  }
}

impl ProtocolHandler for Yiciyuan {
  fn use_latest_output_scheduler(&self) -> bool {
    self.is_fjb02 || self.is_fjb03
  }

  fn handle_stop_output_cmd(
    &self,
    cmd: &CheckedOutputCmdV4,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    // The DeviceHandle calls this only for explicit StopCmd and flushes the
    // result urgently, so it must never apply waveform zero holding.
    self.handle_output_cmd(cmd)
  }

  fn handle_output_oscillate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.handle_axis_cmd(feature_index, speed)
  }

  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.handle_axis_cmd(feature_index, speed)
  }

  fn handle_output_constrict_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.handle_axis_cmd(feature_index, level)
  }

  fn handle_input_subscribe_cmd(
    &self,
    _device_index: u32,
    device: Arc<Hardware>,
    _feature_index: u32,
    feature_id: Uuid,
    sensor_type: InputType,
  ) -> BoxFuture<'_, Result<(), ButtplugDeviceError>> {
    match sensor_type {
      InputType::Battery => {
        async move {
          device
            .subscribe(&HardwareSubscribeCmd::new(
              feature_id,
              Endpoint::RxBLEBattery,
            ))
            .await?;
          Ok(())
        }
      }
      .boxed(),
      _ => future::ready(Err(ButtplugDeviceError::UnhandledCommand(
        "Command not implemented for this sensor".to_string(),
      )))
      .boxed(),
    }
  }

  fn handle_input_unsubscribe_cmd(
    &self,
    device: Arc<Hardware>,
    _feature_index: u32,
    feature_id: Uuid,
    sensor_type: InputType,
  ) -> BoxFuture<'_, Result<(), ButtplugDeviceError>> {
    if (self.is_fjb02 || self.is_fjb03) && sensor_type == InputType::Battery {
      // FF42 also carries protocol ticks. A client ending its battery
      // subscription must not disable the connection-lifetime notification.
      return future::ready(Ok(())).boxed();
    }
    match sensor_type {
      InputType::Battery => {
        async move {
          device
            .unsubscribe(&HardwareUnsubscribeCmd::new(
              feature_id,
              Endpoint::RxBLEBattery,
            ))
            .await?;
          Ok(())
        }
      }
      .boxed(),
      _ => future::ready(Err(ButtplugDeviceError::UnhandledCommand(
        "Command not implemented for this sensor".to_string(),
      )))
      .boxed(),
    }
  }

  fn handle_battery_level_cmd(
    &self,
    device_index: u32,
    device: Arc<Hardware>,
    feature_index: u32,
    feature_id: Uuid,
  ) -> BoxFuture<'_, Result<InputReadingV4, ButtplugDeviceError>> {
    // The cup pushes battery autonomously at ~1Hz as `35 13 01 P C` on the
    // notify characteristic. Subscribe and wait for the first frame whose
    // prefix matches `0x35 0x13`. Other notify frames (uptime ticks
    // `0x35 0x14 ..`, device-info responses `0x35 0x10 ..`) are skipped.
    let mut event_stream = device.event_stream();
    async move {
      device
        .subscribe(&HardwareSubscribeCmd::new(
          feature_id,
          Endpoint::RxBLEBattery,
        ))
        .await?;
      while let Ok(event) = event_stream.recv().await {
        match event {
          HardwareEvent::Notification(_, endpoint, data) => {
            if endpoint != Endpoint::RxBLEBattery {
              continue;
            }
            // Battery frame layout: [0]=0x35, [1]=0x13, [2]=0x01, [3]=pct.
            if data.len() >= 4 && data[0] == 0x35 && data[1] == 0x13 {
              return Ok(InputReadingV4::new(
                device_index,
                feature_index,
                InputTypeReading::Battery(InputValue::new(data[3])),
              ));
            }
            // Not a battery frame — keep waiting for the next notify.
            continue;
          }
          HardwareEvent::Disconnected(_) => {
            return Err(ButtplugDeviceError::ProtocolSpecificError(
              "Yiciyuan".to_owned(),
              "Yiciyuan device disconnected while waiting for battery push.".to_owned(),
            ));
          }
        }
      }
      Err(ButtplugDeviceError::ProtocolSpecificError(
        "Yiciyuan".to_owned(),
        "Yiciyuan device event stream closed before battery push arrived.".to_owned(),
      ))
    }
    .boxed()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::device::hardware::{HardwareInternal, HardwareReadCmd, HardwareReading};
  use buttplug_core::message::{OutputCommand, OutputValue};
  use std::sync::Mutex;
  use tokio::sync::broadcast;

  struct InitHardware {
    calls: Arc<Mutex<Vec<HardwareCommand>>>,
    events: broadcast::Sender<HardwareEvent>,
    failure: u8, // 1: subscribe error, 2: write error, 3: subscribe stalls
  }

  impl HardwareInternal for InitHardware {
    fn disconnect(&self) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
      future::ready(Ok(())).boxed()
    }
    fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
      self.events.subscribe()
    }
    fn read_value(
      &self,
      msg: &HardwareReadCmd,
    ) -> BoxFuture<'static, Result<HardwareReading, ButtplugDeviceError>> {
      future::ready(Ok(HardwareReading::new(msg.endpoint(), &[]))).boxed()
    }
    fn write_value(
      &self,
      msg: &HardwareWriteCmd,
    ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
      self.calls.lock().unwrap().push(msg.clone().into());
      future::ready(if self.failure == 2 {
        Err(ButtplugDeviceError::DeviceCommunicationError(
          "test write failure".into(),
        ))
      } else {
        Ok(())
      })
      .boxed()
    }
    fn subscribe(
      &self,
      msg: &HardwareSubscribeCmd,
    ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
      self.calls.lock().unwrap().push(msg.clone().into());
      if self.failure == 3 {
        return future::pending().boxed();
      }
      future::ready(if self.failure == 1 {
        Err(ButtplugDeviceError::DeviceCommunicationError(
          "test subscribe failure".into(),
        ))
      } else {
        Ok(())
      })
      .boxed()
    }
    fn unsubscribe(
      &self,
      msg: &HardwareUnsubscribeCmd,
    ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
      self.calls.lock().unwrap().push(msg.clone().into());
      future::ready(Ok(())).boxed()
    }
  }

  fn init_hardware(name: &str, failure: u8) -> (Arc<Hardware>, Arc<Mutex<Vec<HardwareCommand>>>) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let (events, _) = broadcast::channel(16);
    let hw = Hardware::new(
      name,
      "test",
      &[Endpoint::Tx, Endpoint::RxBLEBattery],
      &None,
      false,
      Box::new(InitHardware {
        calls: calls.clone(),
        events,
        failure,
      }),
    );
    (Arc::new(hw), calls)
  }

  #[tokio::test]
  async fn fjb02_initialization_subscribes_then_queries_without_motor_output() {
    let (hw, calls) = init_hardware("YCY-FJB-02", 0);
    initialize_fjb(&hw, Duration::ZERO, Duration::from_secs(1))
      .await
      .unwrap();
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert!(
      matches!(&calls[0], HardwareCommand::Subscribe(cmd) if cmd.endpoint() == Endpoint::RxBLEBattery)
    );
    match &calls[1] {
      HardwareCommand::Write(cmd) => {
        assert_eq!(cmd.endpoint(), Endpoint::Tx);
        assert_eq!(
          cmd.data(),
          &vec![0x35, 0x10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert!(!cmd.write_with_response());
      }
      _ => panic!("Expected device-info query"),
    }
  }

  #[tokio::test]
  async fn fjb03_initialization_uses_checksummed_query_without_motor_output() {
    let (hw, calls) = init_hardware("YCY-FJB-03", 0);
    initialize_fjb(&hw, Duration::ZERO, Duration::from_secs(1))
      .await
      .unwrap();
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert!(
      matches!(&calls[0], HardwareCommand::Subscribe(cmd) if cmd.endpoint() == Endpoint::RxBLEBattery)
    );
    match &calls[1] {
      HardwareCommand::Write(cmd) => {
        assert_eq!(cmd.endpoint(), Endpoint::Tx);
        assert_eq!(cmd.data(), &vec![0x35, 0x10, 0, 0, 0, 0x45]);
        assert!(!cmd.write_with_response());
      }
      _ => panic!("Expected device-info query"),
    }
  }

  #[tokio::test]
  async fn fjb_initialization_propagates_errors_and_times_out() {
    for name in ["YCY-FJB-02", "YCY-FJB-03"] {
      for failure in [1, 2, 3] {
        let (hw, calls) = init_hardware(name, failure);
        let result = initialize_fjb(&hw, Duration::ZERO, Duration::from_millis(50)).await;
        assert!(result.is_err());
        assert_eq!(
          calls.lock().unwrap().len(),
          if failure == 2 { 2 } else { 1 }
        );
      }
    }
  }

  #[tokio::test]
  async fn other_models_do_not_run_fjb_initialization() {
    use buttplug_server_device_config::ServerDeviceDefinitionBuilder;
    let definition = ServerDeviceDefinitionBuilder::new("test", &Uuid::new_v4()).finish();
    for name in ["YCY-FJB-01", "Unknown"] {
      let (hw, calls) = init_hardware(name, 0);
      YiciyuanInitializer::default()
        .initialize(hw, &definition)
        .await
        .unwrap();
      assert!(calls.lock().unwrap().is_empty());
    }
  }

  #[tokio::test]
  async fn fjb_battery_unsubscribe_keeps_protocol_notifications() {
    for name in ["YCY-FJB-02", "YCY-FJB-03", "YCY-FJB-01"] {
      let (hw, calls) = init_hardware(name, 0);
      let mut device = fjb03();
      device.is_fjb03 = name == "YCY-FJB-03";
      device.is_fjb02 = name == "YCY-FJB-02";
      device
        .handle_input_unsubscribe_cmd(hw, 1, Uuid::new_v4(), InputType::Battery)
        .await
        .unwrap();
      let calls = calls.lock().unwrap();
      if name == "YCY-FJB-01" {
        assert!(matches!(&calls[..], [HardwareCommand::Unsubscribe(_)]));
      } else {
        assert!(calls.is_empty());
      }
    }
  }

  fn fjb03() -> Yiciyuan {
    Yiciyuan {
      is_fjb03: true,
      is_fjb02: false,
      stroke: AtomicU8::new(0),
      motor_b: AtomicU8::new(0),
      axis_c: AtomicU8::new(0),
    }
  }

  fn packet_data(commands: Vec<HardwareCommand>) -> Vec<Vec<u8>> {
    commands
      .into_iter()
      .map(|command| match command {
        HardwareCommand::Write(write) => write.data().clone(),
        other => panic!("Expected a write command, got {other:?}"),
      })
      .collect()
  }

  #[test]
  fn fjb03_c_mode_uses_fixed_mode_command() {
    let device = fjb03();
    assert_eq!(
      packet_data(device.handle_axis_cmd(FEATURE_AXIS_C, 1).unwrap()),
      vec![vec![0x35, 0x11, 0x04, 0x01, 0x4B]]
    );
    assert_eq!(
      packet_data(device.handle_axis_cmd(FEATURE_AXIS_C, 100).unwrap()),
      vec![vec![0x35, 0x11, 0x04, 0x07, 0x51]]
    );
  }

  #[test]
  fn fjb03_reapplies_vibration_after_stroke_or_suction_changes() {
    let device = fjb03();
    device.handle_axis_cmd(FEATURE_AXIS_C, 50).unwrap();
    assert_eq!(
      packet_data(device.handle_axis_cmd(FEATURE_STROKE, 50).unwrap()),
      vec![
        vec![0x35, 0x12, 0x0A, 0x00, 0x00, 0x51],
        vec![0x35, 0x11, 0x04, 0x04, 0x4E],
      ]
    );
    assert_eq!(
      packet_data(device.handle_axis_cmd(FEATURE_B, 50).unwrap()),
      vec![
        vec![0x35, 0x12, 0x0A, 0x0A, 0x00, 0x5B],
        vec![0x35, 0x11, 0x04, 0x04, 0x4E],
      ]
    );
  }

  #[test]
  fn fjb03_stops_every_motor_in_any_feature_order() {
    let stop_orders = [
      [FEATURE_STROKE, FEATURE_B, FEATURE_AXIS_C],
      [FEATURE_STROKE, FEATURE_AXIS_C, FEATURE_B],
      [FEATURE_B, FEATURE_STROKE, FEATURE_AXIS_C],
      [FEATURE_B, FEATURE_AXIS_C, FEATURE_STROKE],
      [FEATURE_AXIS_C, FEATURE_STROKE, FEATURE_B],
      [FEATURE_AXIS_C, FEATURE_B, FEATURE_STROKE],
    ];
    for stop_order in stop_orders {
      let device = fjb03();
      for feature in [FEATURE_STROKE, FEATURE_B, FEATURE_AXIS_C] {
        device.handle_axis_cmd(feature, 75).unwrap();
      }
      let mut writes = Vec::new();
      for feature in stop_order {
        writes.extend(packet_data(device.handle_axis_cmd(feature, 0).unwrap()));
      }
      assert_eq!(device.stroke.load(Ordering::Relaxed), 0);
      assert_eq!(device.motor_b.load(Ordering::Relaxed), 0);
      assert_eq!(device.axis_c.load(Ordering::Relaxed), 0);
      assert!(writes.contains(&vec![0x35, 0x11, 0x04, 0x00, 0x4A]));
      assert_eq!(
        writes.last(),
        Some(&vec![0x35, 0x12, 0x00, 0x00, 0x00, 0x47])
      );
    }
  }

  #[test]
  fn fjb03_stroke_maps_level_linearly_and_stops_immediately() {
    let device = fjb03();
    assert_eq!(
      packet_data(device.handle_axis_cmd(FEATURE_STROKE, 10).unwrap()),
      vec![vec![0x35, 0x12, 0x02, 0x00, 0x00, 0x49]]
    );
    assert_eq!(
      packet_data(device.handle_axis_cmd(FEATURE_STROKE, 80).unwrap()),
      vec![vec![0x35, 0x12, 0x10, 0x00, 0x00, 0x57]]
    );
    assert_eq!(
      packet_data(device.handle_axis_cmd(FEATURE_STROKE, 0).unwrap()),
      vec![vec![0x35, 0x12, 0x00, 0x00, 0x00, 0x47]]
    );
    assert_eq!(
      packet_data(device.handle_axis_cmd(FEATURE_STROKE, 60).unwrap()),
      vec![vec![0x35, 0x12, 0x0C, 0x00, 0x00, 0x53]]
    );
  }

  #[test]
  fn fjb03_protocol_emits_unsigned_amplitude_for_host_direction_controller() {
    let device = fjb03();
    let mut previous = 0;
    for value in 0..=100 {
      let packets = packet_data(device.handle_axis_cmd(FEATURE_STROKE, value).unwrap());
      let packet = &packets[0];
      assert_eq!(packet.len(), 6);
      assert!(packet[2] >= previous && packet[2] <= 20);
      assert_eq!(
        packet[5],
        packet[..5].iter().copied().fold(0u8, u8::wrapping_add)
      );
      previous = packet[2];
    }
    assert_eq!(previous, 20);
    for value in [101, u32::MAX] {
      assert_eq!(
        packet_data(device.handle_axis_cmd(FEATURE_STROKE, value).unwrap())[0],
        vec![0x35, 0x12, 20, 0, 0, 0x5B]
      );
    }
    for (value, expected) in [(49, 10), (50, 10), (51, 10), (55, 11)] {
      assert_eq!(
        packet_data(device.handle_axis_cmd(FEATURE_STROKE, value).unwrap())[0][2],
        expected
      );
    }
  }

  #[test]
  fn fjb03_live_and_c_mode_packets_do_not_overlap_in_batching() {
    let device = fjb03();
    let live = device
      .handle_axis_cmd(FEATURE_STROKE, 50)
      .unwrap()
      .remove(0);
    let c_mode = device
      .handle_axis_cmd(FEATURE_AXIS_C, 50)
      .unwrap()
      .remove(0);
    assert!(!live.overlaps(&c_mode));
  }

  #[test]
  fn fjb03_never_holds_zero_and_explicit_stop_still_works() {
    let device = fjb03();
    let feature_id = Uuid::new_v4();
    let zero = CheckedOutputCmdV4::new(
      1,
      0,
      FEATURE_STROKE,
      feature_id,
      OutputCommand::Oscillate(OutputValue::new(0)),
    );
    assert_eq!(
      packet_data(device.handle_stop_output_cmd(&zero).unwrap()),
      vec![vec![0x35, 0x12, 0x00, 0x00, 0x00, 0x47]]
    );
  }
}
