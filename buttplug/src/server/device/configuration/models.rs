use std::str::FromStr;

use diesel::prelude::*;
use getset::Getters;
use uuid::Uuid;
use crate::core::message::Endpoint;

use super::schema::*;

#[derive(Queryable, QueryableByName, Selectable, Insertable, Identifiable, Getters, Debug, Eq, PartialEq, Clone)]
#[diesel(table_name = protocol)]
pub struct Protocol {
  #[getset(get_copy="pub")]
  id: i32,
  #[getset(get="pub")]
  protocol_name: String,
  #[getset(get="pub")]
  protocol_display_name: String,
}

#[derive(Queryable, QueryableByName, Selectable, Insertable, Associations, Identifiable, Getters, Debug)]
#[diesel(belongs_to(Protocol))]
#[diesel(table_name = comm_bluetooth_name)]
pub struct CommBluetoothName {
  #[getset(get_copy="pub")]
  id: i32,
  #[getset(get_copy="pub")]
  protocol_id: i32,
  #[getset(get="pub")]
  bluetooth_name: String,
}

#[derive(Queryable, QueryableByName, Selectable, Insertable, Associations, Identifiable, Getters, Debug)]
#[diesel(belongs_to(Protocol))]
#[diesel(table_name = comm_bluetooth_prefix)]
pub struct CommBluetoothPrefix {
  #[getset(get_copy="pub")]
  id: i32,
  #[getset(get_copy="pub")]
  protocol_id: i32,
  #[getset(get="pub")]
  bluetooth_prefix: String,
}

#[derive(Queryable, Associations, Selectable, Insertable, Identifiable, Getters, Debug, Eq, PartialEq, Clone)]
#[diesel(belongs_to(Protocol))]
#[diesel(table_name = comm_bluetooth_service)]
pub struct CommBluetoothService {
  #[getset(get_copy="pub")]
  id: i32,
  //#[getset(get_copy="pub")]
  protocol_id: i32,
  service_uuid: String
}

impl CommBluetoothService {
  pub fn service_uuid(&self) -> Uuid {
    Uuid::parse_str(&self.service_uuid).unwrap()
  }

  pub fn protocol_id(&self) -> i32 {
    self.protocol_id
  }
}

#[derive(Queryable, Selectable, Identifiable, Associations, Getters, Debug, Clone)]
#[diesel(belongs_to(CommBluetoothService))]
#[diesel(table_name = comm_bluetooth_characteristic)]
pub struct CommBluetoothCharacteristic {
  #[getset(get_copy="pub")]
  id: i32,
  #[getset(get_copy="pub")]
  comm_bluetooth_service_id: i32,
  endpoint: String,
  characteristic_uuid: String
}

impl CommBluetoothCharacteristic {
  pub fn endpoint(&self) -> Endpoint {
    Endpoint::from_str(&self.endpoint).unwrap()
  }

  pub fn service_uuid(&self) -> Uuid {
    Uuid::parse_str(&self.characteristic_uuid).unwrap()
  }
}

#[derive(Queryable, QueryableByName, Selectable, Insertable, Identifiable, Getters, Debug)]
#[diesel(table_name = feature_type)]
pub struct FeatureType {
  #[getset(get_copy="pub")]
  id: i32,
  #[getset(get_copy="pub")]
  typename: String,
}