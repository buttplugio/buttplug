use diesel::{
  prelude::*,
  sql_types::Integer,
  Connection,
  sqlite::SqliteConnection, sql_query, RunQueryDsl
};
use futures_util::SinkExt;
use super::{
  schema::{
    self,
    comm_bluetooth_service,
  },
  models::{
    Protocol, CommBluetoothService, CommBluetoothCharacteristic,
  }
};
use crate::{
  core::message::{Endpoint, ActuatorType, ButtplugDeviceCommandMessageUnion}, server::device::{ServerDeviceIdentifier, configuration::{schema::{comm_bluetooth_name, comm_bluetooth_prefix, protocol}, models::CommBluetoothName}}
};
use getset::{Getters, MutGetters, Setters};
use std::{collections::{HashMap, HashSet}, sync::Arc};
use uuid::Uuid;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Getters, MutGetters, Setters)]
#[getset(get = "pub", set = "pub", get_mut = "pub(crate)")]
struct BluetoothLEManufacturerData {
  company: u16,
  data: Option<Vec<u8>>,
}

impl BluetoothLEManufacturerData {
  pub fn new(company: u16, data: &Option<Vec<u8>>) -> Self {
    Self {
      company,
      data: data.clone(),
    }
  }
}

#[derive(Getters)]
#[getset(get = "pub")]
struct BluetoothLEIdentifier {  
  name: String,
  manufacturer_data: Vec<BluetoothLEManufacturerData>
}

impl BluetoothLEIdentifier {
  pub fn new(name: &str, manufacturer_data: &Vec<BluetoothLEManufacturerData>) -> Self {
    Self {
      name: name.to_owned(),
      manufacturer_data: manufacturer_data.clone()
    }
  }
}

pub enum DeviceIdentifier {
  //BluetoothLE(DBBluetoothLEIdentifier),
  Serial(String),
  WebsocketDevice(String),
  HID {vendor_id: u16, product_id: u16},
  XInput(),
  LovenseConnect(),
  LovenseDongle(),
}

pub struct UserDeviceFeature {
  /// Database User Device Feature Id
  user_device_feature_id: u32,
  descriptor: String,
  feature_type: ActuatorType,
  base_range_min: i32,
  base_range_max: i32,
  user_range_min: i32,
  user_range_max: i32,
  feature_messages: ButtplugDeviceCommandMessageUnion
}

pub struct UserDevice {
  /// Database Device Id
  user_device_id: u32,

}

pub struct DeviceDatabaseManagerBuilder {
  // TODO Specify external Device DB

  // TODO Specify external User DB
}

pub struct DeviceDatabaseManager {
  db_connection: Mutex<SqliteConnection>
}

#[derive(QueryableByName, Getters)]
struct ProtocolId {
  #[diesel(sql_type = Integer)]
  #[getset(get = "pub")]
  protocol_id: i32
}

impl DeviceDatabaseManager {
  pub fn new() -> Self {
    // TODO Make utility to write db file to temp directory.
    Self {
      db_connection: Mutex::new(SqliteConnection::establish("c:\\Users\\qdot\\code\\buttplug\\buttplug\\buttplug-device-config\\buttplug-device-config.sqlite").unwrap())
    }
  }

  pub async fn get_or_create_device(&self, identifier: &ServerDeviceIdentifier) {
    // First, find the base device record using the protocol and identifier.

    // See if we have a user device version with the address.

    // If we don't, create it now.

    // Once we have the user device record, check to see if there are any user device features

    // Return the full device specification
  }

  pub fn find_bluetooth_info(&self, name: &str, /* manufacturer_data: Vec<BluetoothLEManufacturerData> */) -> Option<Vec<(Protocol, Vec<(CommBluetoothService, Vec<CommBluetoothCharacteristic>)>)>> {
    //let mut conn = self.db_connection.lock().await;
    let mut conn = SqliteConnection::establish("c:\\Users\\qdot\\code\\buttplug\\buttplug\\buttplug-device-config\\buttplug-device-config.sqlite").unwrap();

    let matched_names_query = comm_bluetooth_name::table
      .filter(comm_bluetooth_name::bluetooth_name.eq(name))
      .select(comm_bluetooth_name::dsl::protocol_id)
      .distinct()
      .into_boxed();

    let matched_prefixes_query = comm_bluetooth_prefix::table
      .filter(diesel::dsl::sql::<diesel::sql_types::Bool>(&format!("\"{name}\" LIKE bluetooth_prefix || '%'")))
      .select(comm_bluetooth_prefix::dsl::protocol_id)
      .distinct()
      .into_boxed();

    let matched_protocols: Vec<Protocol> = protocol::table
      .filter(protocol::dsl::id.eq_any(matched_names_query))
      .or_filter(protocol::dsl::id.eq_any(matched_prefixes_query))
      .select(Protocol::as_select())
      .load(&mut conn)
      .unwrap();

    // Log found protocols
    if matched_protocols.is_empty() {
      return None;
    }

    // Select services and child characteristics
    let services = comm_bluetooth_service::table
      .filter(comm_bluetooth_service::protocol_id.eq_any(matched_protocols.iter().map(|x| x.id())))
      .select(CommBluetoothService::as_select())
      .load(&mut conn)
      .unwrap();

    let characteristics = CommBluetoothCharacteristic::belonging_to(&services)
      .select(CommBluetoothCharacteristic::as_select())
      .load(&mut conn)
      .unwrap();

    // Having gotten all the info, turn it into something useful to send back to the hardware connector
    let protocol_services = services
      .clone()
      .grouped_by(&matched_protocols)
      .into_iter()
      .zip(matched_protocols)
      .map(|(srv, pro)| (pro, srv))
      .collect::<Vec<(Protocol, Vec<CommBluetoothService>)>>();
 
    let service_characteristics: Vec<(CommBluetoothService, Vec<CommBluetoothCharacteristic>)> = characteristics
      .grouped_by(&services)
      .into_iter()
      .zip(services)
      .map(|(chr, srv)| (srv, chr))
      .collect::<Vec<(CommBluetoothService, Vec<CommBluetoothCharacteristic>)>>();
 
    let protocol_characteristics = protocol_services
      .iter()
      .map(|(pro, srv_vec)| {
        (pro.clone(), srv_vec.iter().map(|srv| {
          (srv.clone(), service_characteristics
            .iter()
            .map(|(srv, chrs)| (srv.protocol_id(), chrs.clone()))
            .collect::<HashMap<i32, Vec<CommBluetoothCharacteristic>>>()
            .get(pro.id())
            .unwrap()
            .clone())
        }).collect::<Vec<(CommBluetoothService, Vec<CommBluetoothCharacteristic>)>>())
      })
      .collect::<Vec<(Protocol, Vec<(CommBluetoothService, Vec<CommBluetoothCharacteristic>)>)>>();

    println!("{:?}", protocol_characteristics);
    Some(protocol_characteristics)
  }

}