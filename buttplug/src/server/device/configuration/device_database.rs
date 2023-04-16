use diesel::{
  prelude::*,
  sql_types::Integer,
  Connection,
  sqlite::SqliteConnection, sql_query, RunQueryDsl
};
use super::{
  schema::{
    protocol,
    protocol_bluetooth_service,
  },
  models::{
    Protocol, ProtocolBluetoothService, ProtocolBluetoothCharacteristic,
  }
};
use crate::{
  core::message::Endpoint
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

  pub async fn find_device_info(&self, identifier: &DeviceIdentifier) -> Option<bool> {
    match identifier {
      /*
      DeviceIdentifier::BluetoothLE(ident) => {
        None //ident.name()
      },
      */
      _ => None
    }
  }

  pub fn find_bluetooth_info(&self, name: &str, /* manufacturer_data: Vec<BluetoothLEManufacturerData> */) -> Option<Vec<(Protocol, Vec<(ProtocolBluetoothService, Vec<ProtocolBluetoothCharacteristic>)>)>> {
    //let mut conn = self.db_connection.lock().await;
    let mut conn = SqliteConnection::establish("c:\\Users\\qdot\\code\\buttplug\\buttplug\\buttplug-device-config\\buttplug-device-config.sqlite").unwrap();
    // See if any names or prefixes match. Due to requiring SQLite's string methods, we have to run
    // this as a raw SQL command instead of running the query builder.
    let matched_protocols = 
      sql_query(format!("SELECT * 
      FROM protocol
      WHERE id IN 
        (SELECT protocol_id 
          FROM protocol_bluetooth_name 
          WHERE (prefix = 1 AND bluetooth_name LIKE substr(\"{name}\", 0, LENGTH(bluetooth_name)+1)) 
          OR (prefix = 0 AND bluetooth_name LIKE \"{name}\"));"))
          .load::<Protocol>(&mut conn).unwrap();
    // Log found protocols

    if matched_protocols.is_empty() {
      return None;
    }

    // Select services and child characteristics
    let services = protocol_bluetooth_service::table
      .filter(protocol_bluetooth_service::protocol_id.eq_any(matched_protocols.iter().map(|x| x.id())))
      .select(ProtocolBluetoothService::as_select())
      .load(&mut conn)
      .unwrap();

    let characteristics = ProtocolBluetoothCharacteristic::belonging_to(&services)
      .select(ProtocolBluetoothCharacteristic::as_select())
      .load(&mut conn)
      .unwrap();

    // Having gotten all the info, turn it into something useful to send back to the hardware connector
    let protocol_services = services
      .clone()
      .grouped_by(&matched_protocols)
      .into_iter()
      .zip(matched_protocols)
      .map(|(srv, pro)| (pro, srv))
      .collect::<Vec<(Protocol, Vec<ProtocolBluetoothService>)>>();

    let service_characteristics: Vec<(ProtocolBluetoothService, Vec<ProtocolBluetoothCharacteristic>)> = characteristics
      .grouped_by(&services)
      .into_iter()
      .zip(services)
      .map(|(chr, srv)| (srv, chr))
      .collect::<Vec<(ProtocolBluetoothService, Vec<ProtocolBluetoothCharacteristic>)>>();

    let protocol_characteristics = protocol_services
      .iter()
      .map(|(pro, srv_vec)| {
        (pro.clone(), srv_vec.iter().map(|srv| {
          (srv.clone(), service_characteristics
            .iter()
            .map(|(srv, chrs)| (*srv.id(), chrs.clone()))
            .collect::<HashMap<i32, Vec<ProtocolBluetoothCharacteristic>>>()
            .get(pro.id())
            .unwrap()
            .clone())
        }).collect::<Vec<(ProtocolBluetoothService, Vec<ProtocolBluetoothCharacteristic>)>>())
      })
      .collect::<Vec<(Protocol, Vec<(ProtocolBluetoothService, Vec<ProtocolBluetoothCharacteristic>)>)>>();

    println!("{:?}", protocol_characteristics);
    Some(protocol_characteristics)
  }

}