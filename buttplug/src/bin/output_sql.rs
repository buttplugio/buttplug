use buttplug::{
  server::device::configuration::{
      device_database::DeviceDatabaseManager, 
      ProtocolCommunicationSpecifier, 
      ProtocolAttributesType,
      models::FeatureType,
      schema::{self, feature_type, protocol::dsl::*, comm_bluetooth_name::dsl::*, comm_bluetooth_prefix::dsl::*, comm_bluetooth_service::dsl::*, comm_bluetooth_characteristic::dsl::*, device::dsl::*},
  }, 
  util::device_configuration::load_protocol_configs_no_dm, 
};
use diesel::{prelude::*, Connection, SqliteConnection, insert_into};


fn main() {
  let mut conn = SqliteConnection::establish("c:\\Users\\qdot\\code\\buttplug\\buttplug\\buttplug-device-config\\buttplug-device-config.sqlite").unwrap();
  let config = load_protocol_configs_no_dm(
    None,
    None,
    false,
  ).unwrap();
  for (config_protocol_name, specifiers) in config.protocol_specifiers() {
    let new_protocol_id: i32 = insert_into(protocol)
      .values((protocol_name.eq(config_protocol_name), protocol_display_name.eq(config_protocol_name)))
      .returning(schema::protocol::dsl::id)
      .get_result(&mut conn)
      .unwrap();
    for specifier in specifiers {
      match specifier {
        ProtocolCommunicationSpecifier::BluetoothLE(btle) => {
          // TODO add manufacturer data moves
          for name in btle.names() {
            if name.contains("*") {
              // Remove * character, no longer needed since we have a trait column now.
              let mut config_prefix = name.to_owned();
              config_prefix.pop();
              insert_into(comm_bluetooth_prefix)
                .values((schema::comm_bluetooth_prefix::dsl::protocol_id.eq(new_protocol_id), bluetooth_prefix.eq(config_prefix)))
                .execute(&mut conn)
                .unwrap();
            } else {
              insert_into(comm_bluetooth_name)
                .values((schema::comm_bluetooth_name::dsl::protocol_id.eq(new_protocol_id), bluetooth_name.eq(name)))
                .execute(&mut conn)
                .unwrap();
            }
          }
          for (btle_service_uuid, endpoints) in btle.services() {
            let new_btle_service_id: i32 = insert_into(comm_bluetooth_service)
              .values((schema::comm_bluetooth_service::dsl::protocol_id.eq(new_protocol_id), service_uuid.eq(btle_service_uuid.to_string())))
              .returning(schema::comm_bluetooth_service::dsl::id)
              .get_result(&mut conn)
              .unwrap();
            for (btle_endpoint, char_uuid) in endpoints {
              insert_into(comm_bluetooth_characteristic)
                .values((schema::comm_bluetooth_characteristic::dsl::comm_bluetooth_service_id.eq(new_btle_service_id), endpoint.eq(btle_endpoint.to_string()), characteristic_uuid.eq(char_uuid.to_string())))
                .execute(&mut conn)
                .unwrap();
            }
          }
        },
        // TODO add xinput
        // TODO add hid
        // TODO add serial
        _ => {}
      }
    }
    for (ident, config_device) in config.protocol_attributes() {
      if ident.protocol() != config_protocol_name {
        continue;
      }
      let device_ident = match ident.attributes_identifier() {
        ProtocolAttributesType::Default => None,
        ProtocolAttributesType::Identifier(device_id) => Some(format!("\"{device_id}\"")),
      };
      let config_device_name = config_device.name();
      //println!("INSERT INTO device (id, protocol_id, identifier, device_name) VALUES ({device_id}, {protocol_id}, {device_ident}, \"{device_name}\");");
      let new_device_id: i32 = insert_into(device)
        .values((schema::device::dsl::protocol_id.eq(new_protocol_id), schema::device::dsl::identifier.eq(device_ident), schema::device::dsl::device_name.eq(config_device_name)))
        .returning(schema::device::dsl::id)
        .get_result(&mut conn)
        .unwrap();
      for scalarcmd_attr in config_device.message_attributes().scalar_cmd().as_ref().unwrap_or(&vec![]) {
        let fd = scalarcmd_attr.feature_descriptor();
        let desc = if fd == "N/A" {
          "NULL".to_owned()
        } else {
          format!("\"{fd}\"")
        };
        //let actuator_id = *scalarcmd_attr.actuator_type() as u8;
        let min = scalarcmd_attr.step_range().start();
        let max = scalarcmd_attr.step_range().end();

        let feature_type_id = feature_type::table
          .filter(feature_type::dsl::typename.eq(scalarcmd_attr.actuator_type().to_string()))
          .select(FeatureType::as_select())
          .first(&mut conn)
          .unwrap();

        let new_feature_id: i32 = insert_into(schema::device_feature::dsl::device_feature)
          .values((schema::device_feature::dsl::device_id.eq(new_device_id), schema::device_feature::dsl::feature_type_id.eq(feature_type_id.id()), schema::device_feature::dsl::descriptor.eq(desc), schema::device_feature::dsl::range_min.eq(i32::try_from(*min).unwrap()), schema::device_feature::dsl::range_max.eq(i32::try_from(*max).unwrap())))
          .returning(schema::device_feature::dsl::id)
          .get_result(&mut conn)
          .unwrap();
        
        // ScalarCmd is 1
        insert_into(schema::device_feature_message::dsl::device_feature_message)
          .values((schema::device_feature_message::feature_id.eq(new_feature_id), schema::device_feature_message::feature_message_id.eq(1)))
          .execute(&mut conn)
          .unwrap();
      }
    }
    // TODO Remove inheritance. If an identifier variation has no actuators, copy all from default implementation.
    // TODO Add rotatecmd
    // TODO Add linearcmd
    // TODO Add sensorcmd
  
  }
 
  let mgr = DeviceDatabaseManager::new();
  println!("{:?}", mgr.find_bluetooth_info("XiaoLu"));
  
}