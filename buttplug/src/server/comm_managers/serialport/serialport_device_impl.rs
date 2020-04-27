use crate::{
  core::{errors::ButtplugError, messages::RawReading},
  device::{
    configuration_manager::{DeviceSpecifier, ProtocolDefinition, SerialSpecifier},
    device::{
      BoundedDeviceEventBroadcaster,
      ButtplugDeviceEvent,
      ButtplugDeviceImplCreator,
      DeviceImpl,
      DeviceReadCmd,
      DeviceSubscribeCmd,
      DeviceUnsubscribeCmd,
      DeviceWriteCmd,
    },
    Endpoint,
  },
};
use async_std::{
  prelude::StreamExt,
  sync::{channel, Arc, Mutex, Receiver, Sender},
  task,
};
use async_trait::async_trait;
use broadcaster::BroadcastChannel;
use serialport::{open_with_settings, SerialPort, SerialPortInfo, SerialPortSettings};
use std::{io::ErrorKind, thread, time::Duration};

pub struct SerialPortDeviceImplCreator {
  specifier: DeviceSpecifier,
  port_info: SerialPortInfo,
}

impl SerialPortDeviceImplCreator {
  pub fn new(port_info: &SerialPortInfo) -> Self {
    Self {
      specifier: DeviceSpecifier::Serial(SerialSpecifier::new_from_name(&port_info.port_name)),
      port_info: port_info.clone(),
    }
  }
}

#[async_trait]
impl ButtplugDeviceImplCreator for SerialPortDeviceImplCreator {
  fn get_specifier(&self) -> DeviceSpecifier {
    self.specifier.clone()
  }

  async fn try_create_device_impl(
    &mut self,
    protocol: ProtocolDefinition,
  ) -> Result<Box<dyn DeviceImpl>, ButtplugError> {
    match SerialPortDeviceImpl::new(&self.port_info, protocol) {
      Ok(port) => Ok(Box::new(port)),
      Err(e) => Err(e),
    }
  }
}

fn serial_write_thread(mut port: Box<dyn SerialPort>, mut receiver: Receiver<Vec<u8>>) {
  task::block_on(async move {
    loop {
      match receiver.next().await {
        Some(v) => port.write_all(&v).unwrap(),
        None => break,
      }
    }
  });
}

fn serial_read_thread(mut port: Box<dyn SerialPort>, sender: Sender<Vec<u8>>) {
  loop {
    // TODO This is probably too small
    let mut buf: [u8; 1024] = [0; 1024];
    match port.read(&mut buf) {
      Ok(len) => {
        info!("Got {} serial bytes", len);
        task::block_on(async {
          sender.send(buf[0..len].to_vec()).await;
        });
      }
      Err(e) => {
        if e.kind() == ErrorKind::TimedOut {
          continue;
        }
        error!("{:?}", e);
      }
    }
  }
}

#[derive(Clone)]
pub struct SerialPortDeviceImpl {
  name: String,
  address: String,
  read_thread: Arc<Mutex<thread::JoinHandle<()>>>,
  write_thread: Arc<Mutex<thread::JoinHandle<()>>>,
  // We either have to make our receiver internally mutable, or make
  // EVERYTHING mut across read/write on the trait. So we'll do internal
  // mutability here for now since it already works everywhere else.
  port_receiver: Arc<Mutex<Receiver<Vec<u8>>>>,
  port_sender: Sender<Vec<u8>>,
  port: Arc<Mutex<Box<dyn SerialPort>>>,
  connected: bool,
  event_receiver: BoundedDeviceEventBroadcaster,
}

impl SerialPortDeviceImpl {
  pub fn new(
    port_info: &SerialPortInfo,
    protocol_def: ProtocolDefinition,
  ) -> Result<Self, ButtplugError> {
    // If we've gotten this far, we can expect we have a serial port definition.
    let port_def = protocol_def
      .serial
      .unwrap()
      .into_iter()
      .find(|port| port_info.port_name == port.port)
      .unwrap();
    let mut settings = SerialPortSettings::default();
    settings.baud_rate = port_def.baud_rate;
    // Set our timeout at 10hz. Would be nice if this was async, but oh well.
    settings.timeout = Duration::from_millis(100);
    // TODO for now, assume 8/N/1. Not really sure when/if this would ever change.
    //
    // Mostly just feeling lazy here and don't wanna do the enum conversions.
    /*
    settings.stop_bits = port_def.stop_bits;
    settings.data_bits = port_def.data_bits;
    settings.parity = port_def.parity;
    */
    let port = open_with_settings(&port_info.port_name, &settings).unwrap();

    let (writer_sender, writer_receiver) = channel::<Vec<u8>>(256);
    let (reader_sender, reader_receiver) = channel::<Vec<u8>>(256);

    let read_port = (*port).try_clone().unwrap();
    let read_thread = thread::Builder::new()
      .name("Serial Reader Thread".to_string())
      .spawn(move || {
        serial_read_thread(read_port, reader_sender);
      })
      .unwrap();

    let write_port = (*port).try_clone().unwrap();
    let write_thread = thread::Builder::new()
      .name("Serial Writer Thread".to_string())
      .spawn(move || {
        serial_write_thread(write_port, writer_receiver);
      })
      .unwrap();
    Ok(Self {
      name: port.name().unwrap().to_owned(),
      address: port.name().unwrap().to_owned(),
      read_thread: Arc::new(Mutex::new(read_thread)),
      write_thread: Arc::new(Mutex::new(write_thread)),
      port_receiver: Arc::new(Mutex::new(reader_receiver)),
      port_sender: writer_sender,
      port: Arc::new(Mutex::new(port)),
      connected: true,
      event_receiver: BroadcastChannel::with_cap(256),
    })
  }
}

#[async_trait]
impl DeviceImpl for SerialPortDeviceImpl {
  fn name(&self) -> &str {
    &self.name
  }

  fn address(&self) -> &str {
    &self.address
  }

  fn connected(&self) -> bool {
    self.connected
  }

  fn endpoints(&self) -> Vec<Endpoint> {
    vec![Endpoint::Rx, Endpoint::Tx]
  }

  async fn disconnect(&mut self) {
    self.connected = false;
  }

  fn box_clone(&self) -> Box<dyn DeviceImpl> {
    Box::new((*self).clone())
  }

  fn get_event_receiver(&self) -> BoundedDeviceEventBroadcaster {
    self.event_receiver.clone()
  }

  async fn read_value(&self, _msg: DeviceReadCmd) -> Result<RawReading, ButtplugError> {
    // TODO Should check endpoint validity and length requirements
    let mut receiver = self.port_receiver.lock().await.clone();
    if receiver.is_empty() {
      Ok(RawReading::new(0, Endpoint::Rx, vec![]))
    } else {
      Ok(RawReading::new(
        0,
        Endpoint::Rx,
        receiver.next().await.unwrap(),
      ))
    }
  }

  async fn write_value(&self, msg: DeviceWriteCmd) -> Result<(), ButtplugError> {
    // TODO Should check endpoint validity
    Ok(self.port_sender.send(msg.data).await)
  }

  async fn subscribe(&self, _msg: DeviceSubscribeCmd) -> Result<(), ButtplugError> {
    // TODO Should check endpoint validity
    let mut data_receiver = self.port_receiver.lock().await.clone();
    let event_sender = self.event_receiver.clone();
    task::spawn(async move {
      loop {
        match data_receiver.next().await {
          Some(data) => {
            info!("Got serial data! {:?}", data);
            event_sender
              .send(&ButtplugDeviceEvent::Notification(Endpoint::Tx, data))
              .await
              .unwrap();
          }
          None => {
            info!("Data channel closed, ending serial listener task");
            break;
          }
        }
      }
    });
    Ok(())
  }

  async fn unsubscribe(&self, _msg: DeviceUnsubscribeCmd) -> Result<(), ButtplugError> {
    unimplemented!();
  }
}
