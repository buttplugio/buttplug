use super::{lovense_dongle_device_impl::*, lovense_dongle_messages::*};
use crate::{
  core::{errors::ButtplugError, ButtplugResult},
  server::comm_managers::DeviceCommunicationEvent,
};
use async_channel::{bounded, Receiver, Sender};
use async_trait::async_trait;
use futures::{select, FutureExt, StreamExt};

// I found this hot dog on the ground at
// https://news.ycombinator.com/item?id=22752907 and dusted it off. It still
// tastes fine.
#[async_trait]
pub trait LovenseDongleState: std::fmt::Debug + Sync + Send {
  async fn transition(&mut self) -> Option<Box<dyn LovenseDongleState>>;
}

#[derive(Debug)]
enum IncomingMessage {
  CommMgr(LovenseDeviceCommand),
  Dongle(LovenseDongleIncomingMessage),
  Device(OutgoingLovenseData),
  Disconnect,
}

#[derive(Debug, Clone)]
struct ChannelHub {
  comm_manager_outgoing: Sender<ButtplugResult>,
  comm_manager_incoming: Receiver<LovenseDeviceCommand>,
  dongle_outgoing: Sender<OutgoingLovenseData>,
  dongle_incoming: Receiver<LovenseDongleIncomingMessage>,
  event_outgoing: Sender<DeviceCommunicationEvent>,
}

impl ChannelHub {
  pub fn new(
    comm_manager_outgoing: Sender<ButtplugResult>,
    comm_manager_incoming: Receiver<LovenseDeviceCommand>,
    dongle_outgoing: Sender<OutgoingLovenseData>,
    dongle_incoming: Receiver<LovenseDongleIncomingMessage>,
    event_outgoing: Sender<DeviceCommunicationEvent>,
  ) -> Self {
    Self {
      comm_manager_outgoing,
      comm_manager_incoming,
      dongle_outgoing,
      dongle_incoming,
      event_outgoing,
    }
  }

  pub fn create_new_wait_for_dongle_state(&self) -> Option<Box<dyn LovenseDongleState>> {
    Some(Box::new(LovenseDongleWaitForDongle::new(
      self.comm_manager_incoming.clone(),
      self.comm_manager_outgoing.clone(),
      self.event_outgoing.clone(),
    )))
  }

  pub async fn wait_for_input(&mut self) -> IncomingMessage {
    let mut comm_fut = self.comm_manager_incoming.next().fuse();
    let mut dongle_fut = self.dongle_incoming.next().fuse();
    select! {
      comm_res = comm_fut => {
        match comm_res {
          Some(msg) => IncomingMessage::CommMgr(msg),
          None => {
            error!("Disconnect in comm manager channel, assuming shutdown or catastrophic error, exiting loop");
            IncomingMessage::Disconnect
          }
        }
      }
      dongle_res = dongle_fut => {
        match dongle_res {
          Some(msg) => IncomingMessage::Dongle(msg),
          None => {
            error!("Disconnect in dongle channel, assuming shutdown or disconnect, exiting loop");
            IncomingMessage::Disconnect
          }
        }
      }
    }
  }

  pub async fn wait_for_device_input(
    &mut self,
    mut device_incoming: Receiver<OutgoingLovenseData>,
  ) -> IncomingMessage {
    let mut comm_fut = self.comm_manager_incoming.next().fuse();
    let mut dongle_fut = self.dongle_incoming.next().fuse();
    let mut device_fut = device_incoming.next().fuse();
    select! {
      comm_res = comm_fut => {
        match comm_res {
          Some(msg) => IncomingMessage::CommMgr(msg),
          None => {
            error!("Disconnect in comm manager channel, assuming shutdown or catastrophic error, exiting loop");
            IncomingMessage::Disconnect
          }
        }
      }
      dongle_res = dongle_fut => {
        match dongle_res {
          Some(msg) => IncomingMessage::Dongle(msg),
          None => {
            error!("Disconnect in dongle channel, assuming shutdown or disconnect, exiting loop");
            IncomingMessage::Disconnect
          }
        }
      }
      device_res = device_fut => {
        match device_res {
          Some(msg) => IncomingMessage::Device(msg),
          None => {
            error!("Disconnect in device channel, assuming shutdown or disconnect, exiting loop");
            IncomingMessage::Disconnect
          }
        }
      }
    }
  }

  pub async fn send_output(&self, msg: OutgoingLovenseData) {
    self.dongle_outgoing.send(msg).await.unwrap();
  }

  pub async fn send_event(&self, msg: DeviceCommunicationEvent) {
    self.event_outgoing.send(msg).await.unwrap();
  }
}

pub fn create_lovense_dongle_machine(
  event_outgoing: Sender<DeviceCommunicationEvent>,
  comm_incoming_receiver: Receiver<LovenseDeviceCommand>,
) -> (
  Box<dyn LovenseDongleState>,
  Receiver<Result<(), ButtplugError>>,
) {
  let (comm_outgoing_sender, comm_outgoing_receiver) = bounded(256);
  (
    Box::new(LovenseDongleWaitForDongle::new(
      comm_incoming_receiver,
      comm_outgoing_sender,
      event_outgoing,
    )),
    comm_outgoing_receiver,
  )
}

macro_rules! state_definition {
  ($name:ident) => {
    #[derive(Debug)]
    struct $name {
      hub: ChannelHub,
    }

    impl $name {
      pub fn new(hub: ChannelHub) -> Self {
        Self { hub }
      }
    }
  };
}

macro_rules! device_state_definition {
  ($name:ident) => {
    #[derive(Debug)]
    struct $name {
      hub: ChannelHub,
      device_id: String,
    }

    impl $name {
      pub fn new(hub: ChannelHub, device_id: String) -> Self {
        Self { hub, device_id }
      }
    }
  };
}

#[derive(Debug)]
struct LovenseDongleWaitForDongle {
  comm_receiver: Receiver<LovenseDeviceCommand>,
  comm_sender: Sender<ButtplugResult>,
  event_sender: Sender<DeviceCommunicationEvent>,
}

impl LovenseDongleWaitForDongle {
  pub fn new(
    comm_receiver: Receiver<LovenseDeviceCommand>,
    comm_sender: Sender<ButtplugResult>,
    event_sender: Sender<DeviceCommunicationEvent>,
  ) -> Self {
    Self {
      comm_receiver,
      comm_sender,
      event_sender,
    }
  }
}

#[async_trait]
impl LovenseDongleState for LovenseDongleWaitForDongle {
  async fn transition(&mut self) -> Option<Box<dyn LovenseDongleState>> {
    info!("Running wait for dongle step");
    let mut should_scan = false;
    while let Some(msg) = self.comm_receiver.next().await {
      match msg {
        LovenseDeviceCommand::DongleFound(sender, receiver) => {
          let hub = ChannelHub::new(
            self.comm_sender.clone(),
            self.comm_receiver.clone(),
            sender,
            receiver,
            self.event_sender.clone(),
          );
          if should_scan {
            return Some(Box::new(LovenseDongleStartScanning::new(hub)));
          }
          return Some(Box::new(LovenseDongleIdle::new(hub)));
        }
        LovenseDeviceCommand::StartScanning => {
          should_scan = true;
        }
        LovenseDeviceCommand::StopScanning => {
          should_scan = false;
        }
      }
    }
    None
  }
}

state_definition!(LovenseDongleIdle);

#[async_trait]
impl LovenseDongleState for LovenseDongleIdle {
  async fn transition(&mut self) -> Option<Box<dyn LovenseDongleState>> {
    info!("Running idle step");

    // Check to see if any toy is already connected.
    let autoconnect_msg = LovenseDongleOutgoingMessage {
      func: LovenseDongleMessageFunc::Statuss,
      message_type: LovenseDongleMessageType::Toy,
      id: None,
      command: None,
      eager: None,
    };
    self
      .hub
      .send_output(OutgoingLovenseData::Message(autoconnect_msg))
      .await;

    // This sleep is REQUIRED. If we send too soon after this, the dongle locks up.
    futures_timer::Delay::new(std::time::Duration::from_millis(250)).await;

    loop {
      let msg = self.hub.wait_for_input().await;
      match msg {
        IncomingMessage::Dongle(device_msg) => match device_msg.func {
          LovenseDongleMessageFunc::IncomingStatus => {
            if let Some(incoming_data) = device_msg.data {
              if Some(LovenseDongleResultCode::DeviceConnectSuccess) == incoming_data.status {
                info!("Lovense dongle already connected to a device, registering in system.");
                return Some(Box::new(LovenseDongleDeviceLoop::new(
                  self.hub.clone(),
                  incoming_data.id.unwrap(),
                )));
              }
            }
          }
          _ => error!("Cannot handle dongle function {:?}", device_msg),
        },
        IncomingMessage::CommMgr(comm_msg) => match comm_msg {
          LovenseDeviceCommand::StartScanning => {
            return Some(Box::new(LovenseDongleStartScanning::new(self.hub.clone())));
          }
          LovenseDeviceCommand::StopScanning => {
            return Some(Box::new(LovenseDongleStopScanning::new(self.hub.clone())));
          }
          _ => {
            error!(
              "Unhandled comm manager message to lovense dongle: {:?}",
              comm_msg
            );
          }
        },
        IncomingMessage::Disconnect => {
          error!("Channel disconnect of some kind, returning to 'wait for dongle' state.");
          return self.hub.create_new_wait_for_dongle_state();
        }
        _ => {
          error!("Unhandled message to lovense dongle: {:?}", msg);
        }
      }
    }
  }
}

state_definition!(LovenseDongleStartScanning);

#[async_trait]
impl LovenseDongleState for LovenseDongleStartScanning {
  async fn transition(&mut self) -> Option<Box<dyn LovenseDongleState>> {
    info!("scanning for devices");

    let scan_msg = LovenseDongleOutgoingMessage {
      message_type: LovenseDongleMessageType::Toy,
      func: LovenseDongleMessageFunc::Search,
      eager: None,
      id: None,
      command: None,
    };
    self
      .hub
      .send_output(OutgoingLovenseData::Message(scan_msg))
      .await;
    Some(Box::new(LovenseDongleScanning::new(self.hub.clone())))
  }
}

state_definition!(LovenseDongleScanning);

#[async_trait]
impl LovenseDongleState for LovenseDongleScanning {
  async fn transition(&mut self) -> Option<Box<dyn LovenseDongleState>> {
    info!("scanning for devices");
    loop {
      let msg = self.hub.wait_for_input().await;
      match msg {
        IncomingMessage::CommMgr(comm_msg) => {
          error!("Not handling comm input: {:?}", comm_msg);
        }
        IncomingMessage::Dongle(device_msg) => {
          match device_msg.func {
            LovenseDongleMessageFunc::ToyData => {
              if let Some(data) = device_msg.data {
                return Some(Box::new(LovenseDongleStopScanningAndConnect::new(
                  self.hub.clone(),
                  data.id.unwrap(),
                )));
              } else if device_msg.result.is_some() {
                // emit and return to idle
                return Some(Box::new(LovenseDongleIdle::new(self.hub.clone())));
              }
            }
            _ => error!("Cannot handle dongle function {:?}", device_msg),
          }
        }
        IncomingMessage::Disconnect => {
          error!("Channel disconnect of some kind, returning to 'wait for dongle' state.");
          return self.hub.create_new_wait_for_dongle_state();
        }
        _ => error!("Cannot handle dongle function {:?}", msg),
      }
    }
  }
}

state_definition!(LovenseDongleStopScanning);

#[async_trait]
impl LovenseDongleState for LovenseDongleStopScanning {
  async fn transition(&mut self) -> Option<Box<dyn LovenseDongleState>> {
    info!("stopping search");
    let scan_msg = LovenseDongleOutgoingMessage {
      message_type: LovenseDongleMessageType::USB,
      func: LovenseDongleMessageFunc::StopSearch,
      eager: None,
      id: None,
      command: None,
    };
    self
      .hub
      .send_output(OutgoingLovenseData::Message(scan_msg))
      .await;
    self
      .hub
      .send_event(DeviceCommunicationEvent::ScanningFinished)
      .await;
    None
  }
}

device_state_definition!(LovenseDongleStopScanningAndConnect);

#[async_trait]
impl LovenseDongleState for LovenseDongleStopScanningAndConnect {
  async fn transition(&mut self) -> Option<Box<dyn LovenseDongleState>> {
    info!("stopping search and connecting to device");
    let scan_msg = LovenseDongleOutgoingMessage {
      message_type: LovenseDongleMessageType::USB,
      func: LovenseDongleMessageFunc::StopSearch,
      eager: None,
      id: None,
      command: None,
    };
    self
      .hub
      .send_output(OutgoingLovenseData::Message(scan_msg))
      .await;
    loop {
      let msg = self.hub.wait_for_input().await;
      match msg {
        IncomingMessage::Dongle(device_msg) => match device_msg.func {
          LovenseDongleMessageFunc::Search => {
            if let Some(result) = device_msg.result {
              if result == LovenseDongleResultCode::SearchStopped {
                break;
              }
            }
          }
          _ => error!("Cannot handle dongle function {:?}", device_msg),
        },
        IncomingMessage::Disconnect => {
          error!("Channel disconnect of some kind, returning to 'wait for dongle' state.");
          return self.hub.create_new_wait_for_dongle_state();
        }
        _ => error!("Cannot handle dongle function {:?}", msg),
      }
    }
    self
      .hub
      .send_event(DeviceCommunicationEvent::ScanningFinished)
      .await;
    Some(Box::new(LovenseDongleDeviceLoop::new(
      self.hub.clone(),
      self.device_id.clone(),
    )))
  }
}

device_state_definition!(LovenseDongleDeviceLoop);

#[async_trait]
impl LovenseDongleState for LovenseDongleDeviceLoop {
  async fn transition(&mut self) -> Option<Box<dyn LovenseDongleState>> {
    info!("Running Lovense Dongle Device Event Loop");
    let (device_write_sender, device_write_receiver) = bounded(256);
    let (device_read_sender, device_read_receiver) = bounded(256);
    self
      .hub
      .send_event(DeviceCommunicationEvent::DeviceFound(Box::new(
        LovenseDongleDeviceImplCreator::new(
          &self.device_id,
          device_write_sender,
          device_read_receiver,
        ),
      )))
      .await;
    loop {
      let msg = self
        .hub
        .wait_for_device_input(device_write_receiver.clone())
        .await;
      match msg {
        IncomingMessage::Device(device_msg) => {
          self.hub.send_output(device_msg).await;
        }
        IncomingMessage::Dongle(dongle_msg) => {
          match dongle_msg.func {
            LovenseDongleMessageFunc::IncomingStatus => {
              if let Some(data) = dongle_msg.data {
                if data.status == Some(LovenseDongleResultCode::DeviceDisconnected) {
                  // Device disconnected, emit and return to idle.
                  return Some(Box::new(LovenseDongleIdle::new(self.hub.clone())));
                }
              }
            }
            _ => device_read_sender.send(dongle_msg).await.unwrap(),
          }
        }
        IncomingMessage::CommMgr(comm_msg) => match comm_msg {
          LovenseDeviceCommand::StartScanning => {
            self
              .hub
              .send_event(DeviceCommunicationEvent::ScanningFinished)
              .await;
          }
          LovenseDeviceCommand::StopScanning => {
            self
              .hub
              .send_event(DeviceCommunicationEvent::ScanningFinished)
              .await;
          }
          _ => error!(
            "Cannot handle communication manager function {:?}",
            comm_msg
          ),
        },
        IncomingMessage::Disconnect => {
          error!("Channel disconnect of some kind, returning to 'wait for dongle' state.");
          return self.hub.create_new_wait_for_dongle_state();
        }
      }
    }
  }
}
