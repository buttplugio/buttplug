// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_core::message::OutputType;
use buttplug_core::util::range::RangeInclusive;
use buttplug_server_device_config::{
  DeviceDefinitionSelection,
  RangeWithLimit,
  SDL_CHANNEL_LEFT_TRIGGER_BASE_ID,
  SDL_CHANNEL_LOW_BASE_ID,
  SDL_CHANNEL_RIGHT_TRIGGER_BASE_ID,
  SDL_MAIN_ONLY_BASE_ID,
  SDL_RUMBLE_AND_TRIGGERS_BASE_ID,
  SDL_TRIGGERS_ONLY_BASE_ID,
  ServerDeviceDefinitionBuilder,
  ServerDeviceFeatureOutput,
  ServerDeviceFeatureOutputValueProperties,
  UserDeviceIdentifier,
  load_protocol_configs,
  save_user_config,
};

fn dcm() -> buttplug_server_device_config::DeviceConfigurationManager {
  load_protocol_configs(&None, &None, false)
    .unwrap()
    .finish()
    .unwrap()
}

#[test]
fn definition_selection_rejects_invalid_base() {
  let dcm = dcm();
  let identifier = UserDeviceIdentifier::new("sdl-gamepad-1", "sdl-gamepad", &None);
  let invalid = dcm.device_definition_with_selection(
    &identifier,
    &DeviceDefinitionSelection::new("sdl-gamepad", Some("__nonexistent-base"), "Pad"),
  );
  assert!(matches!(
    invalid,
    Err(buttplug_server_device_config::ButtplugDeviceConfigError::DeviceSelectionInvalid(_))
  ));
  let mismatch = dcm.device_definition_with_selection(
    &identifier,
    &DeviceDefinitionSelection::new("other-protocol", None, "Pad"),
  );
  assert!(matches!(
    mismatch,
    Err(buttplug_server_device_config::ButtplugDeviceConfigError::DeviceSelectionInvalid(_))
  ));
}

#[test]
fn sdl_selection_idempotent() {
  let dcm = dcm();
  let identifier = UserDeviceIdentifier::new(
    "sdl-gamepad-7",
    "sdl-gamepad",
    &Some("Test Pad 1".to_owned()),
  );
  let selection = DeviceDefinitionSelection::new(
    "sdl-gamepad",
    Some("__sdl-rumble-and-triggers"),
    "Test Pad 1",
  );
  let first = dcm
    .device_definition_with_selection(&identifier, &selection)
    .unwrap();
  let second = dcm
    .device_definition_with_selection(&identifier, &selection)
    .unwrap();
  assert_eq!(first.id(), second.id());
  assert_eq!(first.base_id(), Some(SDL_RUMBLE_AND_TRIGGERS_BASE_ID));
  assert_eq!(first.name(), "Test Pad 1");
  assert_eq!(first.features().len(), 4);
  assert_eq!(
    first
      .features()
      .values()
      .map(|f| f.id())
      .collect::<Vec<_>>(),
    second
      .features()
      .values()
      .map(|f| f.id())
      .collect::<Vec<_>>()
  );
  let cached = dcm.device_definition(&identifier).unwrap();
  assert_eq!(cached.id(), second.id());
  assert_eq!(cached.base_id(), second.base_id());
  assert_eq!(
    cached
      .features()
      .values()
      .map(|f| f.id())
      .collect::<Vec<_>>(),
    second
      .features()
      .values()
      .map(|f| f.id())
      .collect::<Vec<_>>()
  );
}

#[test]
fn sdl_layout_reconciliation_matrix() {
  let dcm = dcm();
  let identifier = UserDeviceIdentifier::new("sdl-gamepad-matrix", "sdl-gamepad", &None);
  let main = dcm.device_definition(&identifier).unwrap();
  let main_id = main.id();
  let low = main.features().get(&0).unwrap().id();
  let high = main.features().get(&1).unwrap().id();
  let both = dcm
    .device_definition_with_selection(
      &identifier,
      &DeviceDefinitionSelection::new(
        "sdl-gamepad",
        Some("__sdl-rumble-and-triggers"),
        "Test Pad 1",
      ),
    )
    .unwrap();
  assert_eq!(both.features().get(&0).unwrap().id(), low);
  assert_eq!(both.features().get(&1).unwrap().id(), high);
  assert_eq!(
    both.features().get(&2).unwrap().base_id,
    Some(SDL_CHANNEL_LEFT_TRIGGER_BASE_ID)
  );
  assert_eq!(
    both.features().get(&3).unwrap().base_id,
    Some(SDL_CHANNEL_RIGHT_TRIGGER_BASE_ID)
  );
  assert_eq!(both.id(), main_id);
  let main_again = dcm
    .device_definition_with_selection(
      &identifier,
      &DeviceDefinitionSelection::new("sdl-gamepad", None, "Test Pad 1"),
    )
    .unwrap();
  assert_eq!(main_again.features().len(), 2);
  assert_eq!(main_again.features().get(&0).unwrap().id(), low);
  assert_eq!(main_again.features().get(&1).unwrap().id(), high);
  let triggers = dcm
    .device_definition_with_selection(
      &identifier,
      &DeviceDefinitionSelection::new("sdl-gamepad", Some("__sdl-triggers-only"), "Test Pad 2"),
    )
    .unwrap();
  assert_eq!(triggers.base_id(), Some(SDL_TRIGGERS_ONLY_BASE_ID));
  assert_eq!(
    triggers.features().get(&0).unwrap().base_id,
    Some(SDL_CHANNEL_LEFT_TRIGGER_BASE_ID)
  );
  assert_eq!(
    triggers.features().get(&1).unwrap().base_id,
    Some(SDL_CHANNEL_RIGHT_TRIGGER_BASE_ID)
  );
  assert_ne!(triggers.features().get(&0).unwrap().id(), low);
  assert_ne!(triggers.features().get(&1).unwrap().id(), high);
  assert_eq!(triggers.name(), "Test Pad 2");
  let restored = dcm
    .device_definition_with_selection(
      &identifier,
      &DeviceDefinitionSelection::new(
        "sdl-gamepad",
        Some("__sdl-rumble-and-triggers"),
        "Test Pad 2",
      ),
    )
    .unwrap();
  assert_ne!(restored.features().get(&0).unwrap().id(), low);
  assert_ne!(restored.features().get(&1).unwrap().id(), high);
  assert_eq!(restored.id(), main_id);
  assert_eq!(
    restored.features().get(&0).unwrap().base_id,
    Some(SDL_CHANNEL_LOW_BASE_ID)
  );
}

#[test]
fn non_sdl_definition_resolution_unchanged() {
  let dcm = dcm();
  let identifier = UserDeviceIdentifier::new("COM1", "tcode-v03", &None);
  assert_eq!(
    dcm.device_definition(&identifier).unwrap().name(),
    "TCode v0.3 (Single Linear Axis)"
  );
  let result = dcm.device_definition_with_selection(
    &identifier,
    &DeviceDefinitionSelection::new("tcode-v03", Some("missing"), "TCode"),
  );
  assert!(matches!(
    result,
    Err(buttplug_server_device_config::ButtplugDeviceConfigError::DeviceSelectionInvalid(_))
  ));
}

fn both_selection(name: &str) -> DeviceDefinitionSelection {
  DeviceDefinitionSelection::new("sdl-gamepad", Some("__sdl-rumble-and-triggers"), name)
}

fn reload_with(saved: String) -> buttplug_server_device_config::DeviceConfigurationManager {
  load_protocol_configs(&None, &Some(saved), false)
    .unwrap()
    .finish()
    .unwrap()
}

#[test]
fn sdl_legacy_config_roundtrip() {
  let dcm = dcm();
  let identifier = UserDeviceIdentifier::new(
    "sdl-gamepad-legacy",
    "sdl-gamepad",
    &Some("Legacy Pad".to_owned()),
  );
  let def = dcm.device_definition(&identifier).unwrap();
  assert_eq!(def.base_id(), Some(SDL_MAIN_ONLY_BASE_ID));
  assert_eq!(def.features().len(), 2);
  // Display-name override plus canonical name as hardware would report it.
  let mut builder = ServerDeviceDefinitionBuilder::from_user(&def);
  builder.display_name(&Some("My Precious Pad".to_owned()));
  dcm.add_user_device_definition(&identifier, &builder.finish());

  let saved = save_user_config(&dcm).unwrap();
  let reloaded = reload_with(saved);

  // Cached user definition reloads with the same identity, features, base and
  // display-name override; canonical name falls back to the base default
  // because names are not serialized.
  let back = reloaded.device_definition(&identifier).unwrap();
  assert_eq!(back.id(), def.id());
  assert_eq!(back.base_id(), Some(SDL_MAIN_ONLY_BASE_ID));
  assert_eq!(back.features().len(), 2);
  assert_eq!(
    back.features().values().map(|f| f.id()).collect::<Vec<_>>(),
    def.features().values().map(|f| f.id()).collect::<Vec<_>>()
  );
  assert_eq!(back.name(), "SDL Gamepad");
  assert_eq!(back.display_name(), &Some("My Precious Pad".to_owned()));

  // A later connection refreshes the canonical name without changing identity.
  let reconnected = reloaded
    .device_definition_with_selection(
      &identifier,
      &DeviceDefinitionSelection::new("sdl-gamepad", None, "Legacy Pad"),
    )
    .unwrap();
  assert_eq!(reconnected.id(), def.id());
  assert_eq!(reconnected.name(), "Legacy Pad");
}

#[test]
fn sdl_selected_config_roundtrip() {
  for (selection, expected_base, expected_features) in [
    (
      both_selection("Selected Pad"),
      SDL_RUMBLE_AND_TRIGGERS_BASE_ID,
      4,
    ),
    (
      DeviceDefinitionSelection::new("sdl-gamepad", Some("__sdl-triggers-only"), "Selected Pad"),
      SDL_TRIGGERS_ONLY_BASE_ID,
      2,
    ),
  ] {
    let dcm = dcm();
    let identifier = UserDeviceIdentifier::new(
      "sdl-gamepad-selected",
      "sdl-gamepad",
      &Some("Selected Pad".to_owned()),
    );
    let def = dcm
      .device_definition_with_selection(&identifier, &selection)
      .unwrap();
    assert_eq!(def.base_id(), Some(expected_base));
    assert_eq!(def.features().len(), expected_features);

    let saved = save_user_config(&dcm).unwrap();
    let reloaded = reload_with(saved);
    let back = reloaded
      .device_definition_with_selection(&identifier, &selection)
      .unwrap();
    assert_eq!(back.id(), def.id());
    assert_eq!(back.base_id(), Some(expected_base));
    assert_eq!(back.protocol_variant(), def.protocol_variant());
    assert_eq!(back.name(), "Selected Pad");
    assert_eq!(
      back.features().values().map(|f| f.id()).collect::<Vec<_>>(),
      def.features().values().map(|f| f.id()).collect::<Vec<_>>()
    );
  }
}

#[test]
fn sdl_description_reconciliation_and_reload_contract() {
  let dcm = dcm();
  let identifier = UserDeviceIdentifier::new(
    "sdl-gamepad-desc",
    "sdl-gamepad",
    &Some("Desc Pad".to_owned()),
  );
  let def = dcm
    .device_definition_with_selection(&identifier, &both_selection("Desc Pad"))
    .unwrap();

  // Customize feature 0 with a deliberately nondefault description, feature 1
  // with a user range limit and a disabled flag.
  let mut builder = ServerDeviceDefinitionBuilder::from_user(&def);
  let mut f0 = def.features().get(&0).unwrap().clone();
  f0.description = "My custom low motor label".to_owned();
  builder.replace_feature(&f0);
  let mut f1 = def.features().get(&1).unwrap().clone();
  f1.output = f1
    .output
    .iter()
    .map(|o| match o {
      ServerDeviceFeatureOutput::Vibrate(props) => {
        ServerDeviceFeatureOutput::Vibrate(ServerDeviceFeatureOutputValueProperties::new(
          RangeWithLimit::new_with_user(
            props.value.base.clone(),
            Some(RangeInclusive::new(0, 30000)),
          ),
          true,
        ))
      }
      other => other.clone(),
    })
    .collect();
  builder.replace_feature(&f1);
  dcm.add_user_device_definition(&identifier, &builder.finish());

  // In-memory reconciliation preserves the nonempty custom description and
  // the user customizations.
  let reconn = dcm
    .device_definition_with_selection(&identifier, &both_selection("Desc Pad"))
    .unwrap();
  assert_eq!(
    reconn.features().get(&0).unwrap().description,
    "My custom low motor label"
  );
  let f1_back = reconn.features().get(&1).unwrap();
  match f1_back.get_output(OutputType::Vibrate).unwrap() {
    ServerDeviceFeatureOutput::Vibrate(props) => {
      assert_eq!(
        (props.value.internal().start(), props.value.internal().end()),
        (0, 30000)
      );
      assert!(props.disabled);
    }
    other => panic!("expected vibrate output, got {other:?}"),
  }

  // Save/load: descriptions are not serialized, so reload uses the selected
  // base's descriptions. User limits and disabled flags persist.
  let saved = save_user_config(&dcm).unwrap();
  let reloaded = reload_with(saved);
  let back = reloaded
    .device_definition_with_selection(&identifier, &both_selection("Desc Pad"))
    .unwrap();
  assert_eq!(
    back.features().get(&0).unwrap().description,
    "Low-frequency rumble"
  );
  let f1_reloaded = back.features().get(&1).unwrap();
  match f1_reloaded.get_output(OutputType::Vibrate).unwrap() {
    ServerDeviceFeatureOutput::Vibrate(props) => {
      assert_eq!(
        (props.value.internal().start(), props.value.internal().end()),
        (0, 30000)
      );
      assert!(props.disabled);
    }
    other => panic!("expected vibrate output, got {other:?}"),
  }
}
