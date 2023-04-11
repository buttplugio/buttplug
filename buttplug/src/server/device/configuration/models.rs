use std::str::FromStr;

use diesel::prelude::*;
use getset::{Getters};
use uuid::Uuid;
use crate::core::message::Endpoint;

use super::schema::*;

#[derive(Queryable, QueryableByName, Selectable, Getters, Debug)]
#[diesel(table_name = protocol)]
pub struct Protocol {
  #[getset(get_copy="pub")]
  id: i32,
  #[getset(get="pub")]
  protocol_name: String,
  #[getset(get="pub")]
  display_name: String,
}

#[derive(Queryable, QueryableByName, Associations, Identifiable, Selectable, Getters, Debug)]
#[diesel(belongs_to(Protocol))]
#[diesel(table_name = protocol_bluetooth_name)]
pub struct ProtocolBluetoothName {
  #[getset(get_copy="pub")]
  id: i32,
  #[getset(get_copy="pub")]
  protocol_id: i32,
  #[getset(get="pub")]
  bluetooth_name: String,
}

#[derive(Queryable, QueryableByName, Associations, Identifiable, Selectable, Getters, Debug)]
#[diesel(belongs_to(Protocol))]
#[diesel(table_name = protocol_bluetooth_prefix)]
pub struct ProtocolBluetoothPrefix {
  #[getset(get_copy="pub")]
  id: i32,
  #[getset(get_copy="pub")]
  protocol_id: i32,
  #[getset(get="pub")]
  prefix: String,
}

#[derive(Queryable, Associations, Selectable, Identifiable, Getters, Debug)]
#[diesel(belongs_to(Protocol))]
#[diesel(table_name = protocol_bluetooth_service)]
pub struct ProtocolBluetoothService {
  #[getset(get_copy="pub")]
  id: i32,
  #[getset(get_copy="pub")]
  protocol_id: i32,
  service_uuid: String
}

impl ProtocolBluetoothService {
  pub fn service_uuid(&self) -> Uuid {
    Uuid::parse_str(&self.service_uuid).unwrap()
  }
}

#[derive(Queryable, Selectable, Identifiable, Associations, Getters, Debug)]
#[diesel(belongs_to(ProtocolBluetoothService))]
#[diesel(table_name = protocol_bluetooth_characteristic)]
pub struct ProtocolBluetoothCharacteristic {
  #[getset(get_copy="pub")]
  id: i32,
  #[getset(get_copy="pub")]
  protocol_bluetooth_service_id: i32,
  endpoint: String,
  characteristic_uuid: String
}

impl ProtocolBluetoothCharacteristic {
  pub fn endpoint(&self) -> Endpoint {
    Endpoint::from_str(&self.endpoint).unwrap()
  }

  pub fn service_uuid(&self) -> Uuid {
    Uuid::parse_str(&self.characteristic_uuid).unwrap()
  }
}