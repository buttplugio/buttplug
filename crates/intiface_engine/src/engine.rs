use crate::{
  backdoor_server::BackdoorServer,
  buttplug_server::{reset_buttplug_server, run_server, setup_buttplug_server},
  error::IntifaceEngineError,
  frontend::{
    frontend_external_event_loop, frontend_server_event_loop, process_messages::EngineMessage,
    Frontend,
  },
  mdns::IntifaceMdns,
  options::EngineOptions,
  remote_server::ButtplugRemoteServerEvent,
  ButtplugRepeater,
};

use buttplug_server_device_config::{DeviceConfigurationManager, save_user_config};
use futures::{pin_mut, StreamExt};
use once_cell::sync::OnceCell;
use std::{path::Path, sync::Arc, time::Duration};
use tokio::{fs, select};
use tokio_util::sync::CancellationToken;

#[cfg(debug_assertions)]
pub fn maybe_crash_main_thread(options: &EngineOptions) {
  if options.crash_main_thread() {
    panic!("Crashing main thread by request");
  }
}

#[allow(dead_code)]
#[cfg(debug_assertions)]
pub fn maybe_crash_task_thread(options: &EngineOptions) {
  if options.crash_task_thread() {
    tokio::spawn(async {
      tokio::time::sleep(Duration::from_millis(100)).await;
      panic!("Crashing a task thread by request");
    });
  }
}

#[derive(Default)]
pub struct IntifaceEngine {
  stop_token: Arc<CancellationToken>,
  backdoor_server: OnceCell<Arc<BackdoorServer>>,
}

impl IntifaceEngine {
  pub fn backdoor_server(&self) -> Option<Arc<BackdoorServer>> {
    Some(self.backdoor_server.get()?.clone())
  }

  pub async fn run(
    &self,
    options: &EngineOptions,
    frontend: Option<Arc<dyn Frontend>>,
    dcm: &Option<Arc<DeviceConfigurationManager>>,
  ) -> Result<(), IntifaceEngineError> {
    // Set up Frontend
    if let Some(frontend) = &frontend {
      let frontend_loop = frontend_external_event_loop(frontend.clone(), self.stop_token.clone());
      tokio::spawn(async move {
        frontend_loop.await;
      });

      frontend.connect().await.unwrap();
      frontend.send(EngineMessage::EngineStarted {}).await;
    }

    // Set up mDNS
    let _mdns_server = if options.broadcast_server_mdns() {
      // TODO Unregister whenever we have a live connection

      // TODO Support different services for engine versus repeater
      Some(IntifaceMdns::new())
    } else {
      None
    };

    // Set up Repeater (if in repeater mode)
    if options.repeater_mode() {
      info!("Starting repeater");

      let repeater = ButtplugRepeater::new(
        options.repeater_local_port().unwrap(),
        &options.repeater_remote_address().as_ref().unwrap(),
        self.stop_token.child_token(),
      );
      select! {
        _ = self.stop_token.cancelled() => {
          info!("Owner requested process exit, exiting.");
        }
        _ = repeater.listen() => {
          info!("Repeater listener stopped, exiting.");
        }
      };
      if let Some(frontend) = &frontend {
        frontend.send(EngineMessage::EngineStopped {}).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        frontend.disconnect();
      }
      return Ok(());
    }

    // Set up Engine (if in engine mode)

    // At this point we will have received and validated options.

    // Hang out until those listeners get sick of listening.
    info!("Intiface CLI Setup finished, running server tasks until all joined.");
    let mut server = setup_buttplug_server(options, &self.backdoor_server, &dcm).await?;
    let dcm = server
      .server()
      .device_manager()
      .device_configuration_manager()
      .clone();
    if let Some(config_path) = options.user_device_config_path() {
      let stream = server.event_stream();
      {
        let config_path = config_path.to_owned();
        tokio::spawn(async move {
          pin_mut!(stream);
          loop {
            if let Some(event) = stream.next().await {
              match event {
                ButtplugRemoteServerEvent::DeviceAdded {
                  index: _,
                  identifier: _,
                  name: _,
                  display_name: _,
                } => {
                  if let Ok(config_str) = save_user_config(&dcm) {
                    // Should probably at least log if we fail to write the config file
                    let _ = fs::write(&Path::new(&config_path), config_str).await;
                  }
                }
                _ => continue,
              }
            };
          }
        });
      }
    }
    if let Some(frontend) = &frontend {
      frontend.send(EngineMessage::EngineServerCreated {}).await;
      let event_receiver = server.event_stream();
      let frontend_clone = frontend.clone();
      let stop_child_token = self.stop_token.child_token();
      tokio::spawn(async move {
        frontend_server_event_loop(event_receiver, frontend_clone, stop_child_token).await;
      });
    }

    loop {
      let session_connection_token = CancellationToken::new();
      info!("Starting server");

      // Let everything spin up, then try crashing.

      #[cfg(debug_assertions)]
      maybe_crash_main_thread(options);

      let mut exit_requested = false;
      select! {
        _ = self.stop_token.cancelled() => {
          info!("Owner requested process exit, exiting.");
          exit_requested = true;
        }
        result = run_server(&server, options) => {
          match result {
            Ok(_) => info!("Connection dropped, restarting stay open loop."),
            Err(e) => {
              error!("{}", format!("Process Error: {:?}", e));

              if let Some(frontend) = &frontend {
                frontend
                  .send(EngineMessage::EngineError{ error: format!("Process Error: {:?}", e).to_owned()})
                  .await;
              }
            }
          }
        }
      };
      match server.disconnect().await {
        Ok(_) => {
          info!("Client forcefully disconnected from server.");
          if let Some(frontend) = &frontend {
            frontend.send(EngineMessage::ClientDisconnected {}).await;
          }
        }
        Err(_) => info!("Client already disconnected from server."),
      };
      session_connection_token.cancel();
      if exit_requested {
        info!("Breaking out of event loop in order to exit");
        break;
      }
      // We're not exiting, rebuild our server.
      let dm = server.server().device_manager();
      server = reset_buttplug_server(options, &dm, server.event_sender()).await?;
      info!("Server connection dropped, restarting");
    }
    info!("Shutting down server...");
    if let Err(e) = server.shutdown().await {
      error!("Shutdown failed: {:?}", e);
    }
    info!("Exiting");
    if let Some(frontend) = &frontend {
      frontend.send(EngineMessage::EngineStopped {}).await;
      tokio::time::sleep(Duration::from_millis(100)).await;
      frontend.disconnect();
    }
    Ok(())
  }

  pub fn stop(&self) {
    info!("Engine stop called, cancelling token.");
    self.stop_token.cancel();
  }
}
