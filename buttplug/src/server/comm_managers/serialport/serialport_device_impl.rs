use crate::{core::{errors::ButtplugError, messages::RawReading, ButtplugResultFuture}, device::{
    configuration_manager::{DeviceSpecifier, ProtocolDefinition, SerialSpecifier},
    ButtplugDeviceEvent,
    ButtplugDeviceImplCreator,
    DeviceImpl,
    DeviceReadCmd,
    DeviceSubscribeCmd,
    DeviceUnsubscribeCmd,
    DeviceWriteCmd,
    Endpoint,
  }, util::{async_manager, stream::convert_broadcast_receiver_to_stream}};
use tokio::sync::{mpsc, broadcast, Mutex};
use async_trait::async_trait;
use futures::{Stream, future::BoxFuture};
use serialport::{open_with_settings, SerialPort, SerialPortInfo, SerialPortSettings};
use std::{
  io::ErrorKind,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  thread,
  time::Duration,
  fmt::{self, Debug}
};

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

impl Debug for SerialPortDeviceImplCreator {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("SerialPortDeviceImplCreator")
      .field("port_info", &self.port_info)
      .finish()
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

fn serial_write_thread(mut port: Box<dyn SerialPort>, receiver: mpsc::Receiver<Vec<u8>>) {
  let mut recv = receiver;
  while let Some(v) = recv.blocking_recv() {
    port.write_all(&v).unwrap();
  }
}

fn serial_read_thread(mut port: Box<dyn SerialPort>, sender: mpsc::Sender<Vec<u8>>) {
  loop {
    // TODO This is probably too small
    let mut buf: [u8; 1024] = [0; 1024];
    match port.read(&mut buf) {
      Ok(len) => {
        info!("Got {} serial bytes", len);
        if sender.blocking_send(buf[0..len].to_vec()).is_err() {
          error!("Serial port implementation disappeared, exiting read thread.");
          break;
        }
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

pub struct SerialPortDeviceImpl {
  name: String,
  address: String,
  port_receiver: Arc<Mutex<mpsc::Receiver<Vec<u8>>>>,
  port_sender: mpsc::Sender<Vec<u8>>,
  connected: Arc<AtomicBool>,
  event_sender: broadcast::Sender<ButtplugDeviceEvent>,
  // TODO These aren't actually read, do we need to hold them?
  _read_thread: thread::JoinHandle<()>,
  _write_thread: thread::JoinHandle<()>,
  _port: Arc<Mutex<Box<dyn SerialPort>>>,
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

    let (writer_sender, writer_receiver) = mpsc::channel(256);
    let (reader_sender, reader_receiver) = mpsc::channel(256);

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

    let (event_sender, _) = broadcast::channel(256);
    Ok(Self {
      name: port.name().unwrap(),
      address: port.name().unwrap(),
      _read_thread: read_thread,
      _write_thread: write_thread,
      port_receiver: Arc::new(Mutex::new(reader_receiver)),
      port_sender: writer_sender,
      _port: Arc::new(Mutex::new(port)),
      connected: Arc::new(AtomicBool::new(true)),
      event_sender
    })
  }
}

impl DeviceImpl for SerialPortDeviceImpl {
  fn name(&self) -> &str {
    &self.name
  }

  fn address(&self) -> &str {
    &self.address
  }

  fn connected(&self) -> bool {
    self.connected.load(Ordering::SeqCst)
  }

  fn endpoints(&self) -> Vec<Endpoint> {
    vec![Endpoint::Rx, Endpoint::Tx]
  }

  fn disconnect(&self) -> ButtplugResultFuture {
    let connected = self.connected.clone();
    Box::pin(async move {
      connected.store(false, Ordering::SeqCst);
      Ok(())
    })
  }

  fn event_stream(&self) -> Box<dyn Stream<Item = ButtplugDeviceEvent> + Unpin + Send> {
    Box::new(Box::pin(convert_broadcast_receiver_to_stream(self.event_sender.subscribe())))
  }

  fn read_value(
    &self,
    _msg: DeviceReadCmd,
  ) -> BoxFuture<'static, Result<RawReading, ButtplugError>> {
    // TODO Should check endpoint validity and length requirements
    let receiver = self.port_receiver.clone();
    Box::pin(async move {
      let mut recv_mut = receiver.lock().await;
      Ok(RawReading::new(
        0,
        Endpoint::Rx,
        recv_mut.recv().await.unwrap(),
      ))
    })
  }

  fn write_value(&self, msg: DeviceWriteCmd) -> ButtplugResultFuture {
    let sender = self.port_sender.clone();
    // TODO Should check endpoint validity
    Box::pin(async move {
      sender.send(msg.data).await.unwrap();
      Ok(())
    })
  }

  fn subscribe(&self, _msg: DeviceSubscribeCmd) -> ButtplugResultFuture {
    // TODO Should check endpoint validity
    let data_receiver = self.port_receiver.clone();
    let event_sender = self.event_sender.clone();
    Box::pin(async move {
      async_manager::spawn(async move {
        // TODO There's only one subscribable endpoint on a serial port, so we
        // should check to make sure we don't have multiple subscriptions so we
        // don't deadlock.
        let mut data_receiver_mut = data_receiver.lock().await;
        loop {
          match data_receiver_mut.recv().await {
            Some(data) => {
              info!("Got serial data! {:?}", data);
              event_sender
                .send(ButtplugDeviceEvent::Notification(Endpoint::Tx, data))
                .unwrap();
            }
            None => {
              info!("Data channel closed, ending serial listener task");
              break;
            }
          }
        }
      })
      .unwrap();
      Ok(())
    })
  }

  fn unsubscribe(&self, _msg: DeviceUnsubscribeCmd) -> ButtplugResultFuture {
    unimplemented!();
  }
}
