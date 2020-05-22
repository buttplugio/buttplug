use super::{
  ButtplugClientConnector, ButtplugClientConnectorError, ButtplugClientConnectorResult,
  ButtplugRemoteClientConnectorHelper, ButtplugRemoteClientConnectorMessage,
};
use crate::{
  client::ButtplugInternalClientMessageResult,
  core::messages::{ButtplugClientInMessage, ButtplugClientOutMessage, ButtplugMessage},
};
use async_std::{
  sync::{channel, Receiver},
  task,
};
use async_trait::async_trait;
use async_tungstenite::{
  async_std::connect_async_with_tls_connector,
  tungstenite::protocol::Message,
};
use futures_util::{SinkExt, StreamExt};

pub struct AsyncTungsteniteWebsocketClientConnector {
  helper: ButtplugRemoteClientConnectorHelper,
  recv: Option<Receiver<ButtplugClientOutMessage>>,
  address: String,
  bypass_cert_verify: bool,
}

impl AsyncTungsteniteWebsocketClientConnector {
  pub fn new_insecure_connector(address: &str) -> Self {
    Self {
      helper: ButtplugRemoteClientConnectorHelper::default(),
      recv: None,
      address: address.to_owned(),
      bypass_cert_verify: false,
    }
  }

  pub fn new_secure_connector(address: &str, bypass_cert_verify: bool) -> Self {
    Self {
      helper: ButtplugRemoteClientConnectorHelper::default(),
      recv: None,
      address: address.to_owned(),
      bypass_cert_verify,
    }
  }
}

#[async_trait]
impl ButtplugClientConnector for AsyncTungsteniteWebsocketClientConnector {
  async fn connect(&mut self) -> Result<(), ButtplugClientConnectorError> {
    let (client_sender, client_receiver) = channel(256);
    self.recv = Some(client_receiver);
    let (read_future, connector_input_recv, connector_output_sender) =
      self.helper.get_event_loop_future(client_sender);
    let tls_connector = if self.address.contains("wss://") {
      use async_tls::TlsConnector;
      if self.bypass_cert_verify {
        use rustls::ClientConfig;
        use std::sync::Arc;

        mod danger {

          pub struct NoCertificateVerification {}

          impl rustls::ServerCertVerifier for NoCertificateVerification {
            fn verify_server_cert(
              &self,
              _roots: &rustls::RootCertStore,
              _presented_certs: &[rustls::Certificate],
              _dns_name: webpki::DNSNameRef<'_>,
              _ocsp: &[u8],
            ) -> Result<rustls::ServerCertVerified, rustls::TLSError> {
              Ok(rustls::ServerCertVerified::assertion())
            }
          }
        }

        let mut config = ClientConfig::new();
        config
          .dangerous()
          .set_certificate_verifier(Arc::new(danger::NoCertificateVerification {}));
        Some(TlsConnector::from(Arc::new(config)))
      } else {
        Some(TlsConnector::new())
      }
    } else {
      None
    };

    match connect_async_with_tls_connector(&self.address, tls_connector).await {
      Ok((stream, _)) => {
        let (mut writer, mut reader) = stream.split();
        task::spawn(async move {
          while let Some(msg) = connector_input_recv.recv().await {
            let json = msg.as_protocol_json();
            debug!("Sending: {}", json);
            writer
              .send(Message::text(json))
              .await
              .expect("This should never fail?");
          }
        });
        task::spawn(async move {
          while let Some(response) = reader.next().await {
            debug!("Receiving: {:?}", response);
            match response.unwrap() {
              Message::Text(t) => {
                connector_output_sender
                  .send(ButtplugRemoteClientConnectorMessage::Text(t.to_string()))
                  .await;
              }
              Message::Binary(_) => {}
              Message::Ping(_) => {}
              Message::Pong(_) => {}
              Message::Close(_) => {}
            }
          }
        });
        task::spawn(async move {
          read_future.await;
        });
        Ok(())
      }
      Err(e) => Err(ButtplugClientConnectorError::new(&format!("{:?}", e))),
    }
  }

  async fn disconnect(&mut self) -> ButtplugClientConnectorResult {
    self.helper.close().await;
    Ok(())
  }

  async fn send(&mut self, msg: ButtplugClientInMessage) -> ButtplugInternalClientMessageResult {
    self.helper.send(&msg).await
  }

  fn get_event_receiver(&mut self) -> Receiver<ButtplugClientOutMessage> {
    // This will panic if we've already taken the receiver.
    self.recv.take().unwrap()
  }
}
