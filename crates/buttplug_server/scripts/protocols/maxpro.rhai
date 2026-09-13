// MaxPro 2 protocol script.
//
// Port of crates/buttplug_server/src/device/protocol_impl/maxpro.rs:
// single-motor vibration with a trailing checksum byte (wrapping sum of the
// first nine bytes).

fn metadata() {
  #{ "protocol": "maxpro", "api_version": 1 }
}

fn handle_vibrate(index, speed) {
  let data = [
    0x55,
    0x04,
    0x07,
    0xff,
    0xff,
    0x3f,
    speed & 0xff,
    0x5f,
    speed & 0xff,
    0x00,
  ];
  let crc = 0;
  for b in data {
    crc = (crc + b) & 0xff;
  }
  data[9] = crc;

  [
    #{
      "endpoint": "tx",
      "data": data,
    },
  ]
}
