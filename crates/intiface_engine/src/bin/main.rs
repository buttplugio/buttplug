use argh::FromArgs;
use getset::{CopyGetters, Getters};
use intiface_engine::{
  EngineOptions, EngineOptionsBuilder, IntifaceEngine, IntifaceEngineError, IntifaceError,
};
use std::fs;
use tokio::{select, signal::ctrl_c};
use tracing::{debug, info, Level};
use tracing_subscriber::{
  filter::{EnvFilter, LevelFilter},
  layer::SubscriberExt,
  util::SubscriberInitExt,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// command line interface for intiface/buttplug.
///
/// Note: Commands are one word to keep compat with C#/JS executables currently.
#[derive(FromArgs, Getters, CopyGetters)]
pub struct IntifaceCLIArguments {
  // Options that do something then exit
  /// print version and exit.
  #[argh(switch)]
  #[getset(get_copy = "pub")]
  version: bool,

  /// print version and exit.
  #[argh(switch)]
  #[getset(get_copy = "pub")]
  server_version: bool,

  // Options that set up the server networking
  /// if passed, websocket server listens on all interfaces. Otherwise, only
  /// listen on 127.0.0.1.
  #[argh(switch)]
  #[getset(get_copy = "pub")]
  websocket_use_all_interfaces: bool,

  /// insecure port for websocket servers.
  #[argh(option)]
  #[getset(get_copy = "pub")]
  websocket_port: Option<u16>,

  /// insecure address for connecting to websocket servers.
  #[argh(option)]
  #[getset(get = "pub")]
  websocket_client_address: Option<String>,

  // Options that set up communications with intiface GUI
  /// if passed, output json for parent process via websockets
  #[argh(option)]
  #[getset(get_copy = "pub")]
  frontend_websocket_port: Option<u16>,

  // Options that set up Buttplug server parameters
  /// name of server to pass to connecting clients.
  #[argh(option)]
  #[argh(default = "\"Buttplug Server\".to_owned()")]
  #[getset(get = "pub")]
  server_name: String,

  /// path to the device configuration file
  #[argh(option)]
  #[getset(get = "pub")]
  device_config_file: Option<String>,

  /// path to user device configuration file
  #[argh(option)]
  #[getset(get = "pub")]
  user_device_config_file: Option<String>,

  /// ping timeout maximum for server (in milliseconds)
  #[argh(option)]
  #[argh(default = "0")]
  #[getset(get_copy = "pub")]
  max_ping_time: u32,

  /// set log level for output
  #[allow(dead_code)]
  #[argh(option)]
  #[getset(get_copy = "pub")]
  log: Option<Level>,

  /// turn off bluetooth le device support
  #[argh(switch)]
  #[getset(get_copy = "pub")]
  use_bluetooth_le: bool,

  /// turn off serial device support
  #[argh(switch)]
  #[getset(get_copy = "pub")]
  use_serial: bool,

  /// turn off hid device support
  #[allow(dead_code)]
  #[argh(switch)]
  #[getset(get_copy = "pub")]
  use_hid: bool,

  /// turn off lovense dongle serial device support
  #[argh(switch)]
  #[getset(get_copy = "pub")]
  use_lovense_dongle_serial: bool,

  /// turn off lovense dongle hid device support
  #[argh(switch)]
  #[getset(get_copy = "pub")]
  use_lovense_dongle_hid: bool,

  /// turn off xinput gamepad device support (windows only)
  #[argh(switch)]
  #[getset(get_copy = "pub")]
  use_xinput: bool,

  /// turn on lovense connect app device support (off by default)
  #[argh(switch)]
  #[getset(get_copy = "pub")]
  use_lovense_connect: bool,

  /// turn on websocket server device comm manager
  #[argh(switch)]
  #[getset(get_copy = "pub")]
  use_device_websocket_server: bool,

  /// port for device websocket server comm manager (defaults to 54817)
  #[argh(option)]
  #[getset(get_copy = "pub")]
  device_websocket_server_port: Option<u16>,

  /// if set, broadcast server port/service info via mdns
  #[argh(switch)]
  #[getset(get_copy = "pub")]
  broadcast_server_mdns: bool,

  /// mdns suffix, will be appended to instance names for advertised mdns services (optional, ignored if broadcast_mdns is not set)
  #[argh(option)]
  #[getset(get = "pub")]
  mdns_suffix: Option<String>,

  /// if set, use repeater mode instead of engine mode
  #[argh(switch)]
  #[getset(get_copy = "pub")]
  repeater: bool,

  /// if set, use repeater mode instead of engine mode
  #[argh(option)]
  #[getset(get_copy = "pub")]
  repeater_port: Option<u16>,

  /// if set, use repeater mode instead of engine mode
  #[argh(option)]
  #[getset(get = "pub")]
  repeater_remote_address: Option<String>,

  #[cfg(debug_assertions)]
  /// crash the main thread (that holds the runtime)
  #[argh(switch)]
  #[getset(get_copy = "pub")]
  crash_main_thread: bool,

  #[allow(dead_code)]
  #[cfg(debug_assertions)]
  /// crash the task thread (for testing logging/reporting)
  #[argh(switch)]
  #[getset(get_copy = "pub")]
  crash_task_thread: bool,
}

pub fn setup_console_logging(log_level: Option<Level>) {
  if log_level.is_some() {
    tracing_subscriber::registry()
      .with(tracing_subscriber::fmt::layer())
      .with(LevelFilter::from(log_level))
      .try_init()
      .unwrap();
  } else {
    tracing_subscriber::registry()
      .with(tracing_subscriber::fmt::layer())
      .with(
        EnvFilter::try_from_default_env()
          .or_else(|_| EnvFilter::try_new("info"))
          .unwrap(),
      )
      .try_init()
      .unwrap();
  };
  println!("Intiface Server, starting up with stdout output.");
}

impl TryFrom<IntifaceCLIArguments> for EngineOptions {
  type Error = IntifaceError;
  fn try_from(args: IntifaceCLIArguments) -> Result<Self, IntifaceError> {
    let mut builder = EngineOptionsBuilder::default();

    if let Some(deviceconfig) = args.device_config_file() {
      info!(
        "Intiface CLI Options: External Device Config {}",
        deviceconfig
      );
      match fs::read_to_string(deviceconfig) {
        Ok(cfg) => builder.device_config_json(&cfg),
        Err(err) => {
          return Err(IntifaceError::new(&format!(
            "Error opening external device configuration: {:?}",
            err
          )))
        }
      };
    }

    if let Some(userdeviceconfig) = args.user_device_config_file() {
      info!(
        "Intiface CLI Options: User Device Config {}",
        userdeviceconfig
      );
      match fs::read_to_string(userdeviceconfig) {
        Ok(cfg) => builder.user_device_config_json(&cfg),
        Err(err) => {
          return Err(IntifaceError::new(&format!(
            "Error opening user device configuration: {:?}",
            err
          )))
        }
      };
    }

    builder
      .websocket_use_all_interfaces(args.websocket_use_all_interfaces())
      .use_bluetooth_le(args.use_bluetooth_le())
      .use_serial_port(args.use_serial())
      .use_hid(args.use_hid())
      .use_lovense_dongle_serial(args.use_lovense_dongle_serial())
      .use_lovense_dongle_hid(args.use_lovense_dongle_hid())
      .use_xinput(args.use_xinput())
      .use_lovense_connect(args.use_lovense_connect())
      .use_device_websocket_server(args.use_device_websocket_server())
      .max_ping_time(args.max_ping_time())
      .server_name(args.server_name())
      .broadcast_server_mdns(args.broadcast_server_mdns());

    #[cfg(debug_assertions)]
    {
      builder
        .crash_main_thread(args.crash_main_thread())
        .crash_task_thread(args.crash_task_thread());
    }

    if let Some(value) = args.websocket_port() {
      builder.websocket_port(value);
    }
    if let Some(value) = args.websocket_client_address() {
      builder.websocket_client_address(value);
    }
    if let Some(value) = args.frontend_websocket_port() {
      builder.frontend_websocket_port(value);
    }
    if let Some(value) = args.device_websocket_server_port() {
      builder.device_websocket_server_port(value);
    }
    if args.broadcast_server_mdns() {
      if let Some(value) = args.mdns_suffix() {
        builder.mdns_suffix(value);
      }
    }
    Ok(builder.finish())
  }
}

#[tokio::main(flavor = "current_thread")] //#[tokio::main]
async fn main() -> Result<(), IntifaceEngineError> {
  let args: IntifaceCLIArguments = argh::from_env();
  if args.server_version() {
    println!("{}", VERSION);
    return Ok(());
  }

  if args.version() {
    debug!("Server version command sent, printing and exiting.");
    println!(
      "Intiface CLI (Rust Edition) Version {}, Commit {}, Built {}",
      VERSION,
      option_env!("VERGEN_GIT_SHA_SHORT").unwrap_or("unknown"),
      option_env!("VERGEN_BUILD_TIMESTAMP").unwrap_or("unknown")
    );
    return Ok(());
  }

  if args.frontend_websocket_port().is_none() {
    setup_console_logging(args.log());
  }

  let options = EngineOptions::try_from(args).map_err(IntifaceEngineError::from)?;
  let engine = IntifaceEngine::default();
  select! {
    result = engine.run(&options, None, &None) => {
      if let Err(e) = result {
        println!("Server errored while running:");
        println!("{:?}", e);
      }
    }
    _ = ctrl_c() => {
      info!("Control-c hit, exiting.");
      engine.stop();
    }
  }

  Ok(())
}
