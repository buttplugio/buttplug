// Aneros protocol script.
//
// Port of crates/buttplug_server/src/device/protocol_impl/aneros.rs:
// stateless vibration, one packet per motor, command byte 0xF1 + motor index.

fn metadata() {
  #{ "protocol": "aneros", "api_version": 1 }
}

fn handle_vibrate(index, speed) {
  [
    #{
      "endpoint": "tx",
      "data": [0xF1 + index, speed & 0xff],
    },
  ]
}
