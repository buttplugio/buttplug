// Je Joue protocol script.
//
// Port of crates/buttplug_server/src/device/protocol_impl/jejoue.rs:
// stateful two-motor vibration. The pattern byte selects which motors run;
// when motor 0 is stopped the reported speed falls back to motor 1.
//
// All writes carry Je Joue's fixed protocol UUID as their command id, so the
// server attributes the write to the protocol rather than a single feature.

fn metadata() {
  #{ "protocol": "jejoue", "api_version": 1 }
}

fn init_state() {
  #{ "speeds": [0, 0] }
}

fn handle_vibrate(index, speed) {
  this.speeds[index] = speed;

  // Default to both vibes.
  let pattern = 1;

  // Use vibe 1 as speed.
  let out_speed = this.speeds[0];
  let vibe1_running = out_speed > 0;
  let vibe2_running = false;

  // Unless it's zero, then give vibe 2 a chance.
  if !vibe1_running {
    out_speed = this.speeds[1];

    // If we're vibing on 2 only, then change the pattern.
    if out_speed != 0 {
      vibe2_running = true;
      pattern = 3;
    }
  }

  // If we're vibing on 1 only, then change the pattern.
  if pattern == 1 && out_speed != 0 && !vibe2_running {
    pattern = 2;
  }

  [
    #{
      "endpoint": "tx",
      "data": [pattern, out_speed & 0xff],
      "command_ids": ["d3dd2bf5-b029-4bc1-9466-39f82c2e3258"],
    },
  ]
}
