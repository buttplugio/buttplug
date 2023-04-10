use buttplug::{util::device_configuration::{load_protocol_configs_no_dm}, server::device::configuration::{ProtocolCommunicationSpecifier, ProtocolAttributesType}};

fn main() {
  let config = load_protocol_configs_no_dm(
    None,
    None,
    false,
  ).unwrap();
  let mut protocol_id = 1;
  let mut btle_service_id = 1;
  let mut btle_characteristic_id = 1;
  let mut device_id = 1;
  let mut feature_id = 1;
  let mut scalar_attr_id = 1;
  for (protocol, specifiers) in config.protocol_specifiers() {
    println!("INSERT INTO protocol (id, protocol_name, display_name) VALUES ({protocol_id}, \"{protocol}\", \"{protocol}\");");
    for specifier in specifiers {
      match specifier {
        ProtocolCommunicationSpecifier::BluetoothLE(btle) => {
          for name in btle.names() {
            if name.contains("*") {
              println!("INSERT INTO protocol_bluetooth_prefix (protocol_id, prefix) VALUES ({protocol_id}, \"{name}\");");
            } else {
              println!("INSERT INTO protocol_bluetooth_name (protocol_id, bluetooth_name) VALUES ({protocol_id}, \"{name}\");");
            }
          }
          for (service_uuid, endpoints) in btle.services() {
            println!("INSERT INTO protocol_bluetooth_service (id, protocol_id, service_uuid) VALUES ({btle_service_id}, {protocol_id}, \"{service_uuid}\");");
            for (endpoint, char_uuid) in endpoints {
              println!("INSERT INTO protocol_bluetooth_characteristic (id, service_id, endpoint, characteristic_uuid) VALUES ({btle_characteristic_id}, {btle_service_id}, \"{endpoint}\", \"{char_uuid}\");");
              btle_characteristic_id += 1;
            }
            btle_service_id += 1;
          }
          btle_service_id += 1;
        },
        _ => {}
      }
    }
    // While we're here, find all of the devices linked to this protocol and add those to the DB too.
    for (ident, device) in config.protocol_attributes() {
      if ident.protocol() != protocol {
        continue;
      }
      let device_ident = match ident.attributes_identifier() {
        ProtocolAttributesType::Default => "NULL".to_owned(),
        ProtocolAttributesType::Identifier(id) => format!("\"{id}\""),
      };
      let device_name = device.name();
      println!("INSERT INTO device (id, protocol_id, identifier, device_name) VALUES ({device_id}, {protocol_id}, {device_ident}, \"{device_name}\");");
      for scalarcmd_attr in device.message_attributes().scalar_cmd().as_ref().unwrap_or(&vec![]) {
        let fd = scalarcmd_attr.feature_descriptor();
        let desc = if fd == "N/A" {
          "NULL".to_owned()
        } else {
          format!("\"{fd}\"")
        };
        let actuator_id = *scalarcmd_attr.actuator_type() as u8;
        let min = scalarcmd_attr.step_range().start();
        let max = scalarcmd_attr.step_range().end();
        println!("INSERT INTO device_feature (id, device_id) VALUES ({feature_id}, {device_id});");
        println!("INSERT INTO feature_scalarcmd (id, feature_id, actuator_type_id, description, range_min, range_max) VALUES ({scalar_attr_id}, {feature_id}, {actuator_id}, {desc}, {min}, {max});");
        feature_id += 1;
        scalar_attr_id += 1;
      }
      device_id += 1;
    }
  
    protocol_id += 1;
  }
}