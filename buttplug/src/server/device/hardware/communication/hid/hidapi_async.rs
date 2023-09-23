// Copyright 2020 Shift Cryptosecurity AG
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use futures::prelude::*;
use futures::task::SpawnError;
use hidapi::{HidDevice, HidError};
use std::io;
use std::pin::Pin;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HidAsyncDeviceError {
  #[error("libhid failed")]
  HidApi(#[from] HidError),
  #[error("io failed")]
  Io(#[from] io::Error),
  #[error("spawn failed")]
  Spawn(#[from] SpawnError),
}

enum ReadState {
  Idle,
  Busy,
}

struct DeviceInner {
  device: Arc<Mutex<HidDevice>>,
  read_thread: Option<std::thread::JoinHandle<()>>,
  rstate: ReadState,
  data_rx: mpsc::Receiver<Option<[u8; 64]>>, // One message per read
  req_tx: Option<mpsc::Sender<Waker>>,       // One message per expected read
  buffer: Option<[u8; 64]>,
  buffer_pos: usize,
}

pub struct HidAsyncDevice {
  // store an Option so that `close` works
  inner: Option<Arc<Mutex<DeviceInner>>>,
}

impl Clone for HidAsyncDevice {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.as_ref().map(|dev| Arc::clone(&dev)),
    }
  }
}

impl Drop for HidAsyncDevice {
  fn drop(&mut self) {
    //debug!("dropping hid connection");
    if let Some(inner) = self.inner.take() {
      if let Ok(mut guard) = inner.lock() {
        // Take the waker queue and drop it so that the reader thread finihes
        let req_tx = guard.req_tx.take();
        drop(req_tx);

        // Wait for the reader thread to finish
        match guard.read_thread.take() {
          Some(jh) => match jh.join() {
            Ok(_) => info!("device read thread joined"),
            Err(_) => {} //error!("failed to join device read thread"),
          },
          None => {} //error!("already joined"),
        }
      } else {
        //error!("Failed to take lock on device");
      }
    } else {
      //error!("there was no inner");
    }
  }
}

impl HidAsyncDevice {
  pub fn new(device: HidDevice) -> Result<Self, HidAsyncDeviceError> {
    let (data_tx, data_rx) = mpsc::channel();
    let (req_tx, req_rx) = mpsc::channel::<Waker>();
    // set non-blocking so that we can ignore spurious wakeups.
    //device.set_blocking_mode(false);
    // Must be accessed from both inner thread and asyn_write
    let device = Arc::new(Mutex::new(device));
    let jh = std::thread::spawn({
      let device = Arc::clone(&device);
      move || {
        loop {
          // Wait for read request
          //debug!("waiting for request");
          let waker = match req_rx.recv() {
            Ok(waker) => waker,
            Err(_e) => {
              info!("No more wakers, shutting down");
              return;
            }
          };
          //debug!("Got notified");
          match device.lock() {
            Ok(guard) => {
              let mut buf = [0u8; 64];
              //match guard.read_timeout(&mut buf[..], 1000) {
              match guard.read(&mut buf[..]) {
                Err(_) => {
                  //error!("hidapi failed: {}", e);
                  drop(data_tx);
                  waker.wake_by_ref();
                  break;
                }
                Ok(len) => {
                  if len == 0 {
                    data_tx.send(None).unwrap();
                    waker.wake_by_ref();
                    continue;
                  }
                  //debug!("Read data");
                  if let Err(_) = data_tx.send(Some(buf)) {
                    //error!("Sending internally: {}", e);
                    break;
                  }
                  waker.wake_by_ref();
                }
              }
            }
            Err(_) => {
              //error!("Broken lock: {:?}", e);
              return;
            }
          }
        }
      }
    });
    Ok(Self {
      inner: Some(Arc::new(Mutex::new(DeviceInner {
        device,
        read_thread: Some(jh),
        rstate: ReadState::Idle,
        data_rx,
        req_tx: Some(req_tx),
        buffer: None,
        buffer_pos: 0,
      }))),
    })
  }
}

impl AsyncWrite for HidAsyncDevice {
  fn poll_write(
    mut self: Pin<&mut Self>,
    _cx: &mut Context,
    mut buf: &[u8],
  ) -> Poll<Result<usize, io::Error>> {
    let len = buf.len();
    if self.inner.is_none() {
      return Poll::Ready(Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "Cannot poll a closed device",
      )));
    }
    loop {
      let max_len = usize::min(64, buf.len());
      // The hidapi API requires that you put the report ID in the first byte.
      // If you don't use report IDs you must put a 0 there.
      //let mut buf_with_report_id = [0u8; 1 + 64];
      //(&mut buf_with_report_id[1..1 + max_len]).copy_from_slice(&buf[..max_len]);

      //let this: &mut Self = &mut self;
      //debug!("Will write {} bytes: {:?}", buf.len(), &buf[..]);
      match self.inner.as_mut().unwrap().lock() {
        Ok(guard) => match guard.device.lock() {
          Ok(guard) => {
            guard
              .write(&buf[..])
              .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("hidapi failed: {}", e)))?;
            //debug!("Wrote: {:?}", &buf[0..max_len]);
          }
          Err(_) => {} //error!("{:?}", e),
        },
        Err(e) => {
          return Poll::Ready(Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Mutex broken: {:?}", e),
          )))
        }
      }
      buf = &buf[max_len..];
      if buf.is_empty() {
        //debug!("Wrote total {}: {:?}", buf.len(), buf);
        return Poll::Ready(Ok(len));
      }
    }
  }
  fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), io::Error>> {
    Poll::Ready(Ok(()))
  }
  // TODO cleanup read thread...
  fn poll_close(mut self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), io::Error>> {
    let this: &mut Self = &mut self;
    // take the device and drop it
    let _device = this.inner.take();
    Poll::Ready(Ok(()))
  }
}

// Will always read out 64 bytes. Make sure to read out all bytes to avoid trailing bytes in next
// readout.
// Will store all bytes that did not fit in provided buffer and give them next time.
impl AsyncRead for HidAsyncDevice {
  fn poll_read(
    mut self: Pin<&mut Self>,
    cx: &mut Context,
    buf: &mut [u8],
  ) -> Poll<Result<usize, io::Error>> {
    if self.inner.is_none() {
      return Poll::Ready(Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "Cannot poll a closed device",
      )));
    }
    let mut this = self
      .inner
      .as_mut()
      .unwrap()
      .lock()
      .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Mutex broken: {:?}", e)))?;
    loop {
      let waker = cx.waker().clone();
      match this.rstate {
        ReadState::Idle => {
          //debug!("Sending waker");
          if let Some(req_tx) = &mut this.req_tx {
            if let Err(_e) = req_tx.send(waker) {
              //error!("failed to send waker");
            }
          } else {
            return Poll::Ready(Err(io::Error::new(
              io::ErrorKind::InvalidData,
              "Failed internal send",
            )));
          }
          this.rstate = ReadState::Busy;
        }
        ReadState::Busy => {
          // First send any bytes from the previous readout
          if let Some(inner_buf) = this.buffer.take() {
            let len = usize::min(buf.len(), inner_buf.len());
            let inner_slice = &inner_buf[this.buffer_pos..this.buffer_pos + len];
            let buf_slice = &mut buf[..len];
            buf_slice.copy_from_slice(inner_slice);
            // Check if there is more data left
            if this.buffer_pos + inner_slice.len() < inner_buf.len() {
              this.buffer = Some(inner_buf);
              this.buffer_pos += inner_slice.len();
            } else {
              this.rstate = ReadState::Idle;
            }
            return Poll::Ready(Ok(len));
          }

          // Second try to receive more bytes
          let vec = match this.data_rx.try_recv() {
            Ok(Some(vec)) => vec,
            Ok(None) => {
              // end of stream?
              return Poll::Pending;
            }
            Err(e) => match e {
              mpsc::TryRecvError::Disconnected => {
                return Poll::Ready(Err(io::Error::new(
                  io::ErrorKind::Other,
                  "Inner channel dead",
                )));
              }
              mpsc::TryRecvError::Empty => {
                return Poll::Pending;
              }
            },
          };
          //debug!("Read data {:?}", &vec[..]);
          let len = usize::min(vec.len(), buf.len());
          let buf_slice = &mut buf[..len];
          let vec_slice = &vec[..len];
          buf_slice.copy_from_slice(vec_slice);
          if len < vec.len() {
            // If bytes did not fit in buf, store bytes for next readout
            this.buffer = Some(vec);
            this.buffer_pos = 0;
          } else {
            this.rstate = ReadState::Idle;
          }
          //debug!("returning {}", len);
          return Poll::Ready(Ok(len));
        }
      };
    }
  }
}
