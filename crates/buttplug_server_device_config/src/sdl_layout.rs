// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

pub const SDL_PROTOCOL_NAME: &str = "sdl-gamepad";
pub const SDL_MAIN_ONLY_BASE_ID: uuid::Uuid = uuid::uuid!("b35f2adf-16bc-4425-9276-5d191aeaf107");
pub const SDL_RUMBLE_AND_TRIGGERS_BASE_ID: uuid::Uuid =
  uuid::uuid!("c1d2e3f4-3333-4a7b-8c9d-1e2f3a4b5c6d");
pub const SDL_TRIGGERS_ONLY_BASE_ID: uuid::Uuid =
  uuid::uuid!("d2e3f4a5-4444-4b8c-9dae-2f3a4b5c6d7e");
pub const SDL_CHANNEL_LOW_BASE_ID: uuid::Uuid = uuid::uuid!("f56852c8-cb3b-4703-90b6-6291df0c6314");
pub const SDL_CHANNEL_HIGH_BASE_ID: uuid::Uuid =
  uuid::uuid!("e13388f9-a1b6-4c4c-a7b4-c68eeed293d8");
pub const SDL_CHANNEL_LEFT_TRIGGER_BASE_ID: uuid::Uuid =
  uuid::uuid!("a1b2c3d4-1111-4e5f-8a6b-9c0d1e2f3a4b");
pub const SDL_CHANNEL_RIGHT_TRIGGER_BASE_ID: uuid::Uuid =
  uuid::uuid!("b2c3d4e5-2222-4f6a-9b7c-0d1e2f3a4b5c");
pub const SDL_RUMBLE_AND_TRIGGERS_SELECTOR: &str = "__sdl-rumble-and-triggers";
pub const SDL_TRIGGERS_ONLY_SELECTOR: &str = "__sdl-triggers-only";
pub const SDL_RUMBLE_AND_TRIGGERS_VARIANT: &str = "sdl-rumble-and-triggers";
pub const SDL_TRIGGERS_ONLY_VARIANT: &str = "sdl-triggers-only";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdlGamepadLayout {
  MainOnly,
  TriggersOnly,
  MainAndTriggers,
}

impl SdlGamepadLayout {
  pub fn channel_count(self) -> usize {
    match self {
      Self::MainOnly | Self::TriggersOnly => 2,
      Self::MainAndTriggers => 4,
    }
  }

  /// Logical channel slots in fixed low/high/left-trigger/right-trigger order; positions in this
  /// slice are visible feature indexes. Two channels are ambiguous, so layout always comes from
  /// the protocol variant, never from feature count.
  pub fn logical_slots(self) -> &'static [u8] {
    match self {
      Self::MainOnly => &[0, 1],
      Self::TriggersOnly => &[2, 3],
      Self::MainAndTriggers => &[0, 1, 2, 3],
    }
  }

  pub fn from_protocol_variant(variant: Option<&str>) -> Self {
    match variant {
      Some(SDL_RUMBLE_AND_TRIGGERS_VARIANT) => Self::MainAndTriggers,
      Some(SDL_TRIGGERS_ONLY_VARIANT) => Self::TriggersOnly,
      _ => Self::MainOnly,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn channel_count_and_slots() {
    assert_eq!(SdlGamepadLayout::MainOnly.channel_count(), 2);
    assert_eq!(SdlGamepadLayout::MainOnly.logical_slots(), &[0, 1]);
    assert_eq!(SdlGamepadLayout::TriggersOnly.channel_count(), 2);
    assert_eq!(SdlGamepadLayout::TriggersOnly.logical_slots(), &[2, 3]);
    assert_eq!(SdlGamepadLayout::MainAndTriggers.channel_count(), 4);
    assert_eq!(
      SdlGamepadLayout::MainAndTriggers.logical_slots(),
      &[0, 1, 2, 3]
    );
  }

  #[test]
  fn protocol_variant_mapping() {
    assert_eq!(
      SdlGamepadLayout::from_protocol_variant(None),
      SdlGamepadLayout::MainOnly
    );
    assert_eq!(
      SdlGamepadLayout::from_protocol_variant(Some("")),
      SdlGamepadLayout::MainOnly
    );
    assert_eq!(
      SdlGamepadLayout::from_protocol_variant(Some(SDL_TRIGGERS_ONLY_VARIANT)),
      SdlGamepadLayout::TriggersOnly
    );
    assert_eq!(
      SdlGamepadLayout::from_protocol_variant(Some(SDL_RUMBLE_AND_TRIGGERS_VARIANT)),
      SdlGamepadLayout::MainAndTriggers
    );
    assert_eq!(
      SdlGamepadLayout::from_protocol_variant(Some("unknown")),
      SdlGamepadLayout::MainOnly
    );
  }
}
