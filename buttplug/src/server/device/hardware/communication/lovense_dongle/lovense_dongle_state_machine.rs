// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{lovense_dongle_hardware::*, lovense_dongle_messages::*};
use crate::server::device::hardware::communication::HardwareCommunicationManagerEvent;
use async_trait::async_trait;
use futures::{select, FutureExt};
use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc,
};
use tokio::{
  sync::mpsc::{channel, Receiver, Sender},
  time::sleep,
};

// I found this hot dog on the ground at
// https://news.ycombinator.com/item?id=22752907 and dusted it off. It still
// tastes fine.
#[async_trait]
pub trait LovenseDongleState: std::fmt::Debug + Send {
  async fn transition(mut self: Box<Self>) -> Option<Box<dyn LovenseDongleState>>;
}

#[derive(Debug)]
enum IncomingMessage {
  CommMgr(LovenseDeviceCommand),
  Dongle(LovenseDongleIncomingMessage),
  Device(OutgoingLovenseData),
  Disconnect,
}

#[derive(Debug)]
struct ChannelHub {
  comm_manager_incoming: Receiver<LovenseDeviceCommand>,
  dongle_outgoing: Sender<OutgoingLovenseData>,
  dongle_incoming: Receiver<LovenseDongleIncomingMessage>,
  event_outgoing: Sender<HardwareCommunicationManagerEvent>,
  is_scanning: Arc<AtomicBool>,
}

impl ChannelHub {
  pub fn new(
    comm_manager_incoming: Receiver<LovenseDeviceCommand>,
    dongle_outgoing: Sender<OutgoingLovenseData>,
    dongle_incoming: Receiver<LovenseDongleIncomingMessage>,
    event_outgoing: Sender<HardwareCommunicationManagerEvent>,
    is_scanning: Arc<AtomicBool>,
  ) -> Self {
    Self {
      comm_manager_incoming,
      dongle_outgoing,
      dongle_incoming,
      event_outgoing,
      is_scanning,
    }
  }

  pub fn create_new_wait_for_dongle_state(self) -> Option<Box<dyn LovenseDongleState>> {
    self.is_scanning.store(false, Ordering::SeqCst);
    Some(Box::new(LovenseDongleWaitForDongle::new(
      self.comm_manager_incoming,
      self.event_outgoing,
      self.is_scanning,
    )))
  }

  pub async fn wait_for_dongle_input(&mut self) -> IncomingMessage {
    match self.dongle_incoming.recv().await {
      Some(msg) => IncomingMessage::Dongle(msg),
      None => {
        info!("Disconnect in dongle channel, assuming shutdown or disconnect, exiting loop");
        IncomingMessage::Disconnect
      }
    }
  }

  pub async fn wait_for_input(&mut self) -> IncomingMessage {
    select! {
      comm_res = self.comm_manager_incoming.recv().fuse() => {
        match comm_res {
          Some(msg) => IncomingMessage::CommMgr(msg),
          None => {
            info!("Disconnect in comm manager channel, assuming shutdown or catastrophic error, exiting loop");
            IncomingMessage::Disconnect
          }
        }
      }
      dongle_res = self.dongle_incoming.recv().fuse() => {
        match dongle_res {
          Some(msg) => IncomingMessage::Dongle(msg),
          None => {
            info!("Disconnect in dongle channel, assuming shutdown or disconnect, exiting loop");
            IncomingMessage::Disconnect
          }
        }
      }
    }
  }

  pub async fn wait_for_device_input(
    &mut self,
    device_incoming: &mut Receiver<OutgoingLovenseData>,
  ) -> IncomingMessage {
    pin_mut!(device_incoming);
    select! {
      comm_res = self.comm_manager_incoming.recv().fuse() => {
        match comm_res {
          Some(msg) => IncomingMessage::CommMgr(msg),
          None => {
            info!("Disconnect in comm manager channel, assuming shutdown or catastrophic error, exiting loop");
            IncomingMessage::Disconnect
          }
        }
      }
      dongle_res = self.dongle_incoming.recv().fuse() => {
        match dongle_res {
          Some(msg) => IncomingMessage::Dongle(msg),
          None => {
            info!("Disconnect in dongle channel, assuming shutdown or disconnect, exiting loop");
            IncomingMessage::Disconnect
          }
        }
      }
      device_res = device_incoming.recv().fuse() => {
        match device_res {
          Some(msg) => IncomingMessage::Device(msg),
          None => {
            info!("Disconnect in device channel, assuming shutdown or disconnect, exiting loop");
            IncomingMessage::Disconnect
          }
        }
      }
    }
  }

  pub async fn send_output(&self, msg: OutgoingLovenseData) {
    if self
      .dongle_outgoing
      .send(msg)
      .await
      .is_err() {
        warn!("Dongle message sent without owner being alive, assuming shutdown.");
      }
  }

  pub async fn send_event(&self, msg: HardwareCommunicationManagerEvent) {
    if let Err(e) = self.event_outgoing.send(msg).await {
      warn!(
        "Possible error (ignorable if shutting down state machine): {}",
        e
      );
    }
  }

  pub fn set_scanning_status(&self, is_scanning: bool) {
    self.is_scanning.store(is_scanning, Ordering::SeqCst);
  }
}

pub fn create_lovense_dongle_machine(
  event_outgoing: Sender<HardwareCommunicationManagerEvent>,
  comm_incoming_receiver: Receiver<LovenseDeviceCommand>,
  is_scanning: Arc<AtomicBool>,
) -> Box<dyn LovenseDongleState> {
  Box::new(LovenseDongleWaitForDongle::new(
    comm_incoming_receiver,
    event_outgoing,
    is_scanning,
  ))
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
  event_sender: Sender<HardwareCommunicationManagerEvent>,
  is_scanning: Arc<AtomicBool>,
}

impl LovenseDongleWaitForDongle {
  pub fn new(
    comm_receiver: Receiver<LovenseDeviceCommand>,
    event_sender: Sender<HardwareCommunicationManagerEvent>,
    is_scanning: Arc<AtomicBool>,
  ) -> Self {
    Self {
      comm_receiver,
      event_sender,
      is_scanning,
    }
  }
}

#[async_trait]
impl LovenseDongleState for LovenseDongleWaitForDongle {
  async fn transition(mut self: Box<Self>) -> Option<Box<dyn LovenseDongleState>> {
    info!("Running wait for dongle step");
    let mut should_scan = false;
    while let Some(msg) = self.comm_receiver.recv().await {
      match msg {
        LovenseDeviceCommand::DongleFound(sender, receiver) => {
          let hub = ChannelHub::new(
            self.comm_receiver,
            sender,
            receiver,
            self.event_sender.clone(),
            self.is_scanning,
          );
          return Some(Box::new(LovenseCheckForAlreadyConnectedDevice::new(
            hub,
            should_scan,
          )));
        }
        LovenseDeviceCommand::StartScanning => {
          debug!("Lovense dongle not found, storing StartScanning command until found.");
          self.is_scanning.store(true, Ordering::SeqCst);
          should_scan = true;
        }
        LovenseDeviceCommand::StopScanning => {
          debug!("Lovense dongle not found, clearing StartScanning command and emitting ScanningFinished.");
          self.is_scanning.store(false, Ordering::SeqCst);
          should_scan = false;
          // If we were requested to scan and then asked to stop, act like we at least tried.
          if self
            .event_sender
            .send(HardwareCommunicationManagerEvent::ScanningFinished)
            .await
            .is_err() {
              warn!("Dongle message sent without owner being alive, assuming shutdown.");
          }
        }
      }
    }
    info!("Lovense dongle receiver dropped, exiting state machine.");
    None
  }
}

#[derive(Debug)]
struct LovenseCheckForAlreadyConnectedDevice {
  hub: ChannelHub,
  should_scan: bool,
}

impl LovenseCheckForAlreadyConnectedDevice {
  pub fn new(hub: ChannelHub, should_scan: bool) -> Self {
    Self { hub, should_scan }
  }
}

#[async_trait]
impl LovenseDongleState for LovenseCheckForAlreadyConnectedDevice {
  async fn transition(mut self: Box<Self>) -> Option<Box<dyn LovenseDongleState>> {
    info!("Lovense dongle checking for already connected devices");
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
    // This sleep is REQUIRED. If we send something too soon after this, the
    // dongle locks up. The query for already connected devices just returns
    // nothing if there's no device currently connected, so all we can do is wait.
    let mut id = None;
    let fut = self.hub.wait_for_dongle_input();
    select! {
      incoming_msg = fut.fuse() => {
        match incoming_msg {
          IncomingMessage::Dongle(device_msg) =>
            match device_msg.func {
              LovenseDongleMessageFunc::IncomingStatus => {
                if let Some(incoming_data) = device_msg.data {
                  if Some(LovenseDongleResultCode::DeviceConnectSuccess) == incoming_data.status {
                    info!("Lovense dongle already connected to a device, registering in system.");
                    id = incoming_data.id;
                  }
                }
              }
              func => warn!("Cannot handle dongle function {:?}", func),
            }
            _ => warn!("Cannot handle incoming message {:?}", incoming_msg),
        }
      },
      _ = sleep(std::time::Duration::from_millis(250)).fuse() => {
        // noop, just fall thru.
      }
    }
    if let Some(id) = id {
      info!("Lovense dongle found already connected devices");
      return Some(Box::new(LovenseDongleDeviceLoop::new(self.hub, id)));
    }
    if self.should_scan {
      info!("No devices connected to lovense dongle, scanning.");
      return Some(Box::new(LovenseDongleStartScanning::new(self.hub)));
    }
    info!("No devices connected to lovense dongle, idling.");
    return Some(Box::new(LovenseDongleIdle::new(self.hub)));
  }
}

state_definition!(LovenseDongleIdle);
#[async_trait]
impl LovenseDongleState for LovenseDongleIdle {
  async fn transition(mut self: Box<Self>) -> Option<Box<dyn LovenseDongleState>> {
    info!("Running idle step");

    loop {
      match self.hub.wait_for_input().await {
        IncomingMessage::Dongle(device_msg) => match device_msg.func {
          LovenseDongleMessageFunc::IncomingStatus => {
            if let Some(incoming_data) = device_msg.data {
              if let Some(status) = incoming_data.status {
                match status {
                  LovenseDongleResultCode::DeviceConnectSuccess => {
                    info!("Lovense dongle already connected to a device, registering in system.");
                    return Some(Box::new(LovenseDongleDeviceLoop::new(
                      self.hub,
                      incoming_data
                        .id
                        .expect("Dongle protocol shouldn't change, message always has ID."),
                    )));
                  }
                  _ => warn!(
                    "LovenseDongleIdle State cannot handle dongle status {:?}",
                    status
                  ),
                }
              }
            }
          }
          LovenseDongleMessageFunc::Search => {
            if let Some(result) = device_msg.result {
              match result {
                LovenseDongleResultCode::SearchStopped => debug!("Lovense dongle search stopped."),
                _ => warn!(
                  "LovenseDongleIdle State cannot handle search result {:?}",
                  result
                ),
              }
            }
          }
          LovenseDongleMessageFunc::StopSearch => {
            if let Some(result) = device_msg.result {
              match result {
                LovenseDongleResultCode::CommandSuccess => {
                  debug!("Lovense dongle search stop command successful.")
                }
                _ => warn!(
                  "LovenseDongleIdle State cannot handle stop search result {:?}",
                  result
                ),
              }
            }
          }
          _ => error!(
            "LovenseDongleIdle State cannot handle dongle function {:?}",
            device_msg
          ),
        },
        IncomingMessage::CommMgr(comm_msg) => match comm_msg {
          LovenseDeviceCommand::StartScanning => {
            return Some(Box::new(LovenseDongleStartScanning::new(self.hub)));
          }
          LovenseDeviceCommand::StopScanning => {
            return Some(Box::new(LovenseDongleStopScanning::new(self.hub)));
          }
          _ => {
            warn!(
              "Unhandled comm manager message to lovense dongle: {:?}",
              comm_msg
            );
          }
        },
        IncomingMessage::Disconnect => {
          info!("Channel disconnect of some kind, returning to 'wait for dongle' state.");
          return self.hub.create_new_wait_for_dongle_state();
        }
        msg => {
          warn!("Unhandled message to lovense dongle: {:?}", msg);
        }
      }
    }
  }
}

state_definition!(LovenseDongleStartScanning);

#[async_trait]
impl LovenseDongleState for LovenseDongleStartScanning {
  async fn transition(mut self: Box<Self>) -> Option<Box<dyn LovenseDongleState>> {
    debug!("starting scan for devices");

    let scan_msg = LovenseDongleOutgoingMessage {
      message_type: LovenseDongleMessageType::Toy,
      func: LovenseDongleMessageFunc::Search,
      eager: None,
      id: None,
      command: None,
    };
    self.hub.set_scanning_status(true);
    self
      .hub
      .send_output(OutgoingLovenseData::Message(scan_msg))
      .await;
    Some(Box::new(LovenseDongleScanning::new(self.hub)))
  }
}

state_definition!(LovenseDongleScanning);

#[async_trait]
impl LovenseDongleState for LovenseDongleScanning {
  async fn transition(mut self: Box<Self>) -> Option<Box<dyn LovenseDongleState>> {
    debug!("scanning for devices");
    loop {
      let msg = self.hub.wait_for_input().await;
      match msg {
        IncomingMessage::CommMgr(comm_msg) => match comm_msg {
          LovenseDeviceCommand::StopScanning => {
            return Some(Box::new(LovenseDongleStopScanning::new(self.hub)));
          }
          msg => error!("Not handling comm input: {:?}", msg),
        },
        IncomingMessage::Dongle(device_msg) => {
          match device_msg.func {
            LovenseDongleMessageFunc::IncomingStatus => {
              if let Some(incoming_data) = device_msg.data {
                if let Some(status) = incoming_data.status {
                  match status {
                    LovenseDongleResultCode::DeviceConnectSuccess => {
                      info!("Lovense dongle already connected to a device, registering in system.");
                      return Some(Box::new(LovenseDongleDeviceLoop::new(
                        self.hub,
                        incoming_data
                          .id
                          .expect("Dongle protocol shouldn't change, message always has ID."),
                      )));
                    }
                    _ => {
                      warn!(
                        "LovenseDongleScanning state cannot handle dongle status {:?}",
                        status
                      )
                    }
                  }
                }
              }
            }
            LovenseDongleMessageFunc::Search => {
              if let Some(result) = device_msg.result {
                match result {
                  LovenseDongleResultCode::SearchStarted => {
                    debug!("Lovense dongle search started.")
                  }
                  LovenseDongleResultCode::SearchStopped => {
                    debug!(
                      "Lovense dongle stopped scanning before stop was requested, restarting."
                    );
                    return Some(Box::new(LovenseDongleStartScanning::new(self.hub)));
                  }
                  _ => warn!(
                    "LovenseDongleIdle State cannot handle search result {:?}",
                    result
                  ),
                }
              }
            }
            LovenseDongleMessageFunc::ToyData => {
              if let Some(data) = device_msg.data {
                return Some(Box::new(LovenseDongleStopScanningAndConnect::new(
                  self.hub,
                  data
                    .id
                    .expect("Dongle protocol shouldn't change, message always has ID."),
                )));
              } else if device_msg.result.is_some() {
                // emit and return to idle
                return Some(Box::new(LovenseDongleIdle::new(self.hub)));
              }
            }
            _ => warn!(
              "LovenseDongleScanning state cannot handle dongle function {:?}",
              device_msg
            ),
          }
        }
        IncomingMessage::Disconnect => {
          info!("Channel disconnect of some kind, returning to 'wait for dongle' state.");
          self.hub.set_scanning_status(false);
          return self.hub.create_new_wait_for_dongle_state();
        }
        _ => warn!(
          "LovenseDongleScanning state cannot handle dongle function {:?}",
          msg
        ),
      }
    }
  }
}

state_definition!(LovenseDongleStopScanning);

#[async_trait]
impl LovenseDongleState for LovenseDongleStopScanning {
  async fn transition(mut self: Box<Self>) -> Option<Box<dyn LovenseDongleState>> {
    info!("stopping search");
    let scan_msg = LovenseDongleOutgoingMessage {
      message_type: LovenseDongleMessageType::Usb,
      func: LovenseDongleMessageFunc::StopSearch,
      eager: None,
      id: None,
      command: None,
    };
    self
      .hub
      .send_output(OutgoingLovenseData::Message(scan_msg))
      .await;
    self.hub.set_scanning_status(false);
    self
      .hub
      .send_event(HardwareCommunicationManagerEvent::ScanningFinished)
      .await;
    Some(Box::new(LovenseDongleIdle::new(self.hub)))
  }
}

device_state_definition!(LovenseDongleStopScanningAndConnect);

#[async_trait]
impl LovenseDongleState for LovenseDongleStopScanningAndConnect {
  async fn transition(mut self: Box<Self>) -> Option<Box<dyn LovenseDongleState>> {
    info!("stopping search and connecting to device");
    let scan_msg = LovenseDongleOutgoingMessage {
      message_type: LovenseDongleMessageType::Usb,
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
                self.hub.set_scanning_status(false);
                break;
              }
            }
          }
          LovenseDongleMessageFunc::StopSearch => {
            if let Some(result) = device_msg.result {
              if result == LovenseDongleResultCode::CommandSuccess {
                // Just log and continue here.
                debug!("Lovense dongle stop search command succeeded.");
              }
            }
          }
          _ => warn!(
            "LovenseDongleStopScanningAndConnect cannot handle dongle function {:?}",
            device_msg
          ),
        },
        IncomingMessage::Disconnect => {
          info!("Channel disconnect of some kind, returning to 'wait for dongle' state.");
          return self.hub.create_new_wait_for_dongle_state();
        }
        _ => warn!("Cannot handle dongle function {:?}", msg),
      }
    }
    self
      .hub
      .send_event(HardwareCommunicationManagerEvent::ScanningFinished)
      .await;
    Some(Box::new(LovenseDongleDeviceLoop::new(
      self.hub,
      self.device_id.clone(),
    )))
  }
}

device_state_definition!(LovenseDongleDeviceLoop);

#[async_trait]
impl LovenseDongleState for LovenseDongleDeviceLoop {
  async fn transition(mut self: Box<Self>) -> Option<Box<dyn LovenseDongleState>> {
    info!("Running Lovense Dongle Device Event Loop");
    let (device_write_sender, mut device_write_receiver) = channel(256);
    let (device_read_sender, device_read_receiver) = channel(256);
    self
      .hub
      .send_event(HardwareCommunicationManagerEvent::DeviceFound {
        name: "Lovense Dongle Device".to_owned(),
        address: self.device_id.clone(),
        creator: Box::new(LovenseDongleHardwareConnector::new(
          &self.device_id,
          device_write_sender,
          device_read_receiver,
        )),
      })
      .await;
    loop {
      let msg = self
        .hub
        .wait_for_device_input(&mut device_write_receiver)
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
                  return Some(Box::new(LovenseDongleIdle::new(self.hub)));
                }
              }
            }
            _ => {
              if device_read_sender
              .send(dongle_msg)
              .await
              .is_err() {
                warn!("Dongle message sent without owner being alive, assuming shutdown.");
              }
            }
          }
        }
        IncomingMessage::CommMgr(comm_msg) => match comm_msg {
          LovenseDeviceCommand::StartScanning => {
            self.hub.set_scanning_status(false);
            self
              .hub
              .send_event(HardwareCommunicationManagerEvent::ScanningFinished)
              .await;
          }
          LovenseDeviceCommand::StopScanning => {
            self.hub.set_scanning_status(false);
            self
              .hub
              .send_event(HardwareCommunicationManagerEvent::ScanningFinished)
              .await;
          }
          _ => warn!(
            "Cannot handle communication manager function {:?}",
            comm_msg
          ),
        },
        IncomingMessage::Disconnect => {
          info!("Channel disconnect of some kind, returning to 'wait for dongle' state.");
          return self.hub.create_new_wait_for_dongle_state();
        }
      }
    }
  }
}
