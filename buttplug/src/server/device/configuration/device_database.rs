use diesel::{
  prelude::*,
  sql_types::Integer,
  Connection,
  sqlite::SqliteConnection, sql_query, RunQueryDsl
};
use super::{
  schema::{
    protocol_bluetooth_name,
    protocol_bluetooth_prefix, protocol_bluetooth_service, protocol_bluetooth_characteristic,
  },
  models::{
    Protocol, ProtocolBluetoothService, ProtocolBluetoothCharacteristic,
    //ProtocolBluetoothCharacteristic,
    //ProtocolBluetoothService
  }
};
use crate::{
  core::message::Endpoint
};
use getset::{Getters, MutGetters, Setters};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use tokio::sync::Mutex;

// Note: There's a ton of extra structs in here just to deserialize the json
// file. Just leave them and build extras (for instance,
// DeviceProtocolConfiguration) if needed elsewhere in the codebase. It's not
// gonna hurt anything and making a ton of serde attributes is just going to get
// confusing (see the messages impl).

#[derive(Debug, Clone, Getters, MutGetters, Setters)]
#[getset(get = "pub", set = "pub", get_mut = "pub(crate)")]
pub struct DBBluetoothLEManufacturerData {
  company: u16,
  data: Option<Vec<u8>>,
}

impl DBBluetoothLEManufacturerData {
  pub fn new(company: u16, data: &Option<Vec<u8>>) -> Self {
    Self {
      company,
      data: data.clone(),
    }
  }
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

  pub fn find_bluetooth_info(&self, name: &str, manufacturer_data: Vec<DBBluetoothLEManufacturerData>) -> Option<Vec<(ProtocolBluetoothService, Vec<ProtocolBluetoothCharacteristic>)>> {
    //let mut conn = self.db_connection.lock().await;
    let mut conn = SqliteConnection::establish("c:\\Users\\qdot\\code\\buttplug\\buttplug\\buttplug-device-config\\buttplug-device-config.sqlite").unwrap();
    // See if any names or prefixes match. Due to requiring SQLite's string methods, we have to run
    // this as a raw SQL command instead of running the query builder.
    let matched_protocols = 
      sql_query(format!("SELECT protocol_id 
      FROM protocol_bluetooth_name 
      WHERE (prefix = 1 AND bluetooth_name LIKE substr(\"{name}\", 0, LENGTH(bluetooth_name)+1)) 
      OR (prefix = 0 AND bluetooth_name LIKE \"{name}\");"))
          .load::<ProtocolId>(&mut conn).unwrap();
    // Log found protocols

    if matched_protocols.is_empty() {
      return None;
    }

    // Select services and child characteristics
    let services = protocol_bluetooth_service::table
      .filter(protocol_bluetooth_service::protocol_id.eq_any(matched_protocols.iter().map(|x| x.protocol_id())))
      .select(ProtocolBluetoothService::as_select())
      .load(&mut conn)
      .unwrap();

    let characteristics = ProtocolBluetoothCharacteristic::belonging_to(&services)
      .select(ProtocolBluetoothCharacteristic::as_select())
      .load(&mut conn)
      .unwrap();

    Some(characteristics
      .grouped_by(&services)
      .into_iter()
      .zip(services)
      .map(|(chr, srv)| (srv, chr))
      .collect::<Vec<(ProtocolBluetoothService, Vec<ProtocolBluetoothCharacteristic>)>>()
    )
  }
}