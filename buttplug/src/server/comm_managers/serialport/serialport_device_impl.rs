use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::RawReading,
    ButtplugResultFuture,
  },
  device::{
    configuration_manager::{DeviceSpecifier, ProtocolDefinition, SerialSpecifier},
    ButtplugDeviceEvent,
    ButtplugDeviceImplCreator,
    DeviceImpl,
    DeviceImplInternal,
    DeviceReadCmd,
    DeviceSubscribeCmd,
    DeviceUnsubscribeCmd,
    DeviceWriteCmd,
    Endpoint,
  },
  server::comm_managers::ButtplugDeviceSpecificError,
  util::async_manager,
};
use async_trait::async_trait;
use futures::{future::BoxFuture, FutureExt};
use serialport::{SerialPort, SerialPortInfo};
use std::{
  fmt::{self, Debug},
  io::ErrorKind,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  thread,
  time::Duration,
};
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio_util::sync::CancellationToken;

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
  ) -> Result<DeviceImpl, ButtplugError> {
    let device_impl_internal = SerialPortDeviceImpl::try_create(&self.port_info, protocol).await?;
    let device_impl = DeviceImpl::new(
      &self.port_info.port_name,
      &self.port_info.port_name,
      &[Endpoint::Rx, Endpoint::Tx],
      Box::new(device_impl_internal),
    );
    Ok(device_impl)
  }
}

fn serial_write_thread(mut port: Box<dyn SerialPort>, receiver: mpsc::Receiver<Vec<u8>>) {
  let mut recv = receiver;
  // Instead of waiting on a token here, we'll expect that we'll break on our
  // channel going away.
  //
  // This is a blocking recv so we don't have to worry about the port.
  while let Some(v) = recv.blocking_recv() {
    if let Err(err) = port.write_all(&v) {
      error!("Cannot write data to serial port, exiting thread: {}", err);
      return;
    }
  }
}

fn serial_read_thread(
  mut port: Box<dyn SerialPort>,
  sender: mpsc::Sender<Vec<u8>>,
  token: CancellationToken,
) {
  while !token.is_cancelled() {
    // TODO This is probably too small
    let mut buf: [u8; 1024] = [0; 1024];
    match port.bytes_to_read() {
      Ok(read_len) => {
        if read_len == 0 {
          thread::sleep(Duration::from_millis(10));
          continue;
        }
        match port.read(&mut buf) {
          Ok(len) => {
            trace!("Got {} serial bytes", len);
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
      Err(e) => {
        warn!("Error reading from serial port: {:?}", e);
        if e.kind() == serialport::ErrorKind::NoDevice {
          info!("Serial device gone, breaking out of read loop.");
        }
      }
    }
  }
}

pub struct SerialPortDeviceImpl {
  address: String,
  port_receiver: Arc<Mutex<mpsc::Receiver<Vec<u8>>>>,
  port_sender: mpsc::Sender<Vec<u8>>,
  connected: Arc<AtomicBool>,
  device_event_sender: broadcast::Sender<ButtplugDeviceEvent>,
  // TODO These aren't actually read, do we need to hold them?
  _read_thread: thread::JoinHandle<()>,
  _write_thread: thread::JoinHandle<()>,
  _port: Arc<Mutex<Box<dyn SerialPort>>>,
  thread_cancellation_token: CancellationToken,
}

impl SerialPortDeviceImpl {
  pub async fn try_create(
    port_info: &SerialPortInfo,
    protocol_def: ProtocolDefinition,
  ) -> Result<Self, ButtplugError> {
    let (device_event_sender, _) = broadcast::channel(256);
    // If we've gotten this far, we can expect we have a serial port definition.
    let port_def = protocol_def
      .serial
      .expect("This will exist if we've made it here")
      .into_iter()
      .find(|port| port_info.port_name == port.port)
      .expect("We had to match the port already to get here.");

    // This seems like it should be a oneshot, but there's no way to await a
    // value on those?
    let (port_sender, mut port_receiver) = mpsc::channel(1);
    // Mostly just feeling lazy here and don't wanna do the enum conversions.
    /*
    settings.stop_bits = port_def.stop_bits;
    settings.data_bits = port_def.data_bits;
    settings.parity = port_def.parity;
    */
    // TODO for now, assume 8/N/1. Not really sure when/if this would ever change.
    let port_name = port_info.port_name.clone();
    thread::Builder::new()
      .name("Serial Port Connection Thread".to_string())
      .spawn(move || {
        debug!("Starting serial port connection thread for {}", port_name);
        let port_result = serialport::new(&port_name, port_def.baud_rate)
          .timeout(Duration::from_millis(100))
          .open();
        if port_sender.blocking_send(port_result)
          .is_err() {
            warn!("Serial port open thread did not return before serial device was dropped. Dropping port.");
          }
        debug!("Exiting serial port connection thread for {}", port_name);
      })
      .expect("Thread creation should always succeed.");

    let port = port_receiver
      .recv()
      .await
      .expect("This will always be a Some value, we're just blocking for bringup")
      .map_err(|e| {
        ButtplugError::from(ButtplugDeviceError::DeviceSpecificError(
          ButtplugDeviceSpecificError::SerialError(e.to_string()),
        ))
      })?;
    debug!("Serial port received from thread.");
    let (writer_sender, writer_receiver) = mpsc::channel(256);
    let (reader_sender, reader_receiver) = mpsc::channel(256);

    let token = CancellationToken::new();
    let read_token = token.child_token();
    let read_port = (*port)
      .try_clone()
      .expect("Should always be able to clone port");
    let read_thread = thread::Builder::new()
      .name("Serial Reader Thread".to_string())
      .spawn(move || {
        serial_read_thread(read_port, reader_sender, read_token);
      })
      .expect("Should always be able to create thread");

    let write_port = (*port)
      .try_clone()
      .expect("Should always be able to clone port");
    let write_thread = thread::Builder::new()
      .name("Serial Writer Thread".to_string())
      .spawn(move || {
        serial_write_thread(write_port, writer_receiver);
      })
      .expect("Should always be able to create thread");

    Ok(Self {
      address: port
        .name()
        .unwrap_or_else(|| "Default Serial Port Device (No Name Given)".to_owned()),
      _read_thread: read_thread,
      _write_thread: write_thread,
      port_receiver: Arc::new(Mutex::new(reader_receiver)),
      port_sender: writer_sender,
      _port: Arc::new(Mutex::new(port)),
      connected: Arc::new(AtomicBool::new(true)),
      device_event_sender,
      thread_cancellation_token: token,
    })
  }
}

impl DeviceImplInternal for SerialPortDeviceImpl {
  fn event_stream(&self) -> broadcast::Receiver<ButtplugDeviceEvent> {
    self.device_event_sender.subscribe()
  }

  fn connected(&self) -> bool {
    self.connected.load(Ordering::SeqCst)
  }

  fn disconnect(&self) -> ButtplugResultFuture {
    let connected = self.connected.clone();
    Box::pin(async move {
      connected.store(false, Ordering::SeqCst);
      Ok(())
    })
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
        recv_mut
          .recv()
          .now_or_never()
          .unwrap_or_else(|| Some(vec![]))
          .expect("Always set to Some before unwrapping."),
      ))
    })
  }

  fn write_value(&self, msg: DeviceWriteCmd) -> ButtplugResultFuture {
    let sender = self.port_sender.clone();
    // TODO Should check endpoint validity
    Box::pin(async move {
      sender
        .send(msg.data)
        .await
        .expect("Tasks should exist if we get here.");
      Ok(())
    })
  }

  fn subscribe(&self, _msg: DeviceSubscribeCmd) -> ButtplugResultFuture {
    // TODO Should check endpoint validity
    let data_receiver = self.port_receiver.clone();
    let event_sender = self.device_event_sender.clone();
    let address = self.address.clone();
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
                .send(ButtplugDeviceEvent::Notification(
                  address.clone(),
                  Endpoint::Tx,
                  data,
                ))
                .expect("As long as we're subscribed we should have a listener");
            }
            None => {
              info!("Data channel closed, ending serial listener task");
              break;
            }
          }
        }
      });
      Ok(())
    })
  }

  fn unsubscribe(&self, _msg: DeviceUnsubscribeCmd) -> ButtplugResultFuture {
    unimplemented!();
  }
}

impl Drop for SerialPortDeviceImpl {
  fn drop(&mut self) {
    self.thread_cancellation_token.cancel();
  }
}
