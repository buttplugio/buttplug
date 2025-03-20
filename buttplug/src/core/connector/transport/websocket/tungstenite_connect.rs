use std::{
  net::{SocketAddr, ToSocketAddrs},
  sync::Arc,
};

use futures::io::AllowStdIo;
use tokio::task::spawn_blocking;
use tokio_tungstenite::{
  client_async_tls,
  client_async_tls_with_config,
  tungstenite::{
    client::IntoClientRequest,
    error::UrlError,
    handshake::client::Response,
    http::Uri,
    protocol::WebSocketConfig,
    Error,
  },
  Connector,
  MaybeTlsStream,
  WebSocketStream,
};
use tokio_util::compat::{Compat, FuturesAsyncReadCompatExt};

pub type TcpStream = Compat<AllowStdIo<std::net::TcpStream>>;

pub struct TcpListener(Arc<std::net::TcpListener>);

impl TcpListener {
  pub async fn bind<A: ToSocketAddrs>(addr: A) -> Result<Self, std::io::Error> {
    let addr = addr.to_socket_addrs()?.collect::<Vec<SocketAddr>>();
    Ok(Self(Arc::new(
      spawn_blocking(move || std::net::TcpListener::bind(addr.as_slice()))
        .await
        .unwrap()?,
    )))
  }

  pub async fn accept(&self) -> Result<(TcpStream, SocketAddr), std::io::Error> {
    let listener = Arc::clone(&self.0);
    spawn_blocking(move || listener.accept())
      .await
      .unwrap()
      .map(|(stream, addr)| (AllowStdIo::new(stream).compat(), addr))
  }
}

fn connect_uri(uri: &Uri) -> Result<std::net::TcpStream, Error> {
  let host = uri.host().ok_or(Error::Url(UrlError::NoHostName))?;
  let host = if host.starts_with('[') {
    &host[1..host.len() - 1]
  } else {
    host
  };
  let port = uri.port_u16().unwrap_or(match uri.scheme_str() {
    Some("ws") => 80,
    Some("wss") => 443,
    _ => return Err(Error::Url(UrlError::UnsupportedUrlScheme)),
  });
  let addrs = (host, port).to_socket_addrs()?;
  for addr in addrs {
    debug!("Trying to contact {uri} at {addr}...");
    if let Ok(stream) = std::net::TcpStream::connect(addr) {
      return Ok(stream);
    }
  }
  Err(Error::Url(UrlError::UnableToConnect(uri.to_string())))
}

pub async fn connect_async<Req: IntoClientRequest>(
  request: Req,
) -> Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response), Error> {
  let request = request.into_client_request()?;
  let uri = request.uri().clone();
  match spawn_blocking(move || connect_uri(&uri)).await.unwrap() {
    Ok(stream) => {
      stream.set_nodelay(true)?;
      client_async_tls(request, AllowStdIo::new(stream).compat()).await
    }
    Err(e) => Err(e),
  }
}

pub async fn connect_async_tls_with_config<Req: IntoClientRequest>(
  request: Req,
  config: Option<WebSocketConfig>,
  disable_nagle: bool,
  connector: Option<Connector>,
) -> Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response), Error> {
  let request = request.into_client_request()?;
  let uri = request.uri().clone();
  match spawn_blocking(move || connect_uri(&uri)).await.unwrap() {
    Ok(stream) => {
      stream.set_nodelay(disable_nagle)?;
      client_async_tls_with_config(request, AllowStdIo::new(stream).compat(), config, connector)
        .await
    }
    Err(e) => Err(e),
  }
}
