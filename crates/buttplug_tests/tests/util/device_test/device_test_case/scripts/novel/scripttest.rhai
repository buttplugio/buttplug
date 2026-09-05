// Novel test-only protocol for exercising the script protocol test harness.
// The protocol name "scripttest" has no Rust implementation, so a passing
// YAML test case proves the script was loaded and served, with no possible
// fallback to a built-in.

fn metadata() {
  #{ "protocol": "scripttest", "api_version": 1 }
}

fn handle_vibrate(index, speed) {
  [
    #{
      "endpoint": "tx",
      "data": [0xC0, 0xFF, 0xEE, speed & 0xff],
    },
  ]
}
