use buttplug_client_in_process::in_process_client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  // Run this to turn on the environment logger. Running this more than once will panic.
  tracing_subscriber::fmt::init();

  // Now when you connect here, if you've set the RUST_LOG environment variable
  // (set it to "Info" or "Debug"), you should see messages about connection
  // setup.
  let _client = in_process_client("Example Client").await;

  Ok(())
}
