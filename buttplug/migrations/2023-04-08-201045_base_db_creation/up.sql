-- Your SQL goes here

PRAGMA foreign_keys = ON;

CREATE TABLE protocol (
  id INTEGER PRIMARY KEY NOT NULL,
  -- name of the protocol that we'll use inside code
  protocol_name TEXT NOT NULL UNIQUE,
  -- name of the protocol as it will be displayed to the user
  protocol_display_name TEXT NOT NULL
);

CREATE TABLE comm_bluetooth_name (
  id INTEGER PRIMARY KEY NOT NULL,
  protocol_id INTEGER NOT NULL,
  bluetooth_name TEXT NOT NULL,
  FOREIGN KEY(protocol_id)
    REFERENCES protocol (id)
      ON DELETE CASCADE
      ON UPDATE NO ACTION
);

CREATE TABLE comm_bluetooth_prefix (
  id INTEGER PRIMARY KEY NOT NULL,
  protocol_id INTEGER NOT NULL,
  bluetooth_prefix TEXT NOT NULL,
  FOREIGN KEY(protocol_id)
    REFERENCES protocol (id)
      ON DELETE CASCADE
      ON UPDATE NO ACTION
);

CREATE UNIQUE INDEX comm_bluetooth_name_unique ON comm_bluetooth_name(protocol_id, bluetooth_name);

CREATE TABLE comm_bluetooth_manufacturer_data (
  id INTEGER PRIMARY KEY NOT NULL,
  protocol_id INTEGER NOT NULL,
  manufacturer_company INTEGER NOT NULL,
  manufacturer_data BLOB NOT NULL,
  FOREIGN KEY(protocol_id)
    REFERENCES protocol (id)
      ON DELETE CASCADE
      ON UPDATE NO ACTION
);

CREATE TABLE comm_bluetooth_service (
  id INTEGER PRIMARY KEY NOT NULL,
  protocol_id INTEGER NOT NULL,
  service_uuid TEXT NOT NULL,
  FOREIGN KEY(protocol_id)
    REFERENCES protocol (id)
      ON DELETE CASCADE
      ON UPDATE NO ACTION   
);

CREATE UNIQUE INDEX comm_bluetooth_service_unique ON comm_bluetooth_service(protocol_id, service_uuid);

CREATE TABLE comm_bluetooth_characteristic (
  id INTEGER PRIMARY KEY NOT NULL,
  comm_bluetooth_service_id INTEGER NOT NULL,
  endpoint TEXT NOT NULL,
  characteristic_uuid TEXT NOT NULL,
  FOREIGN KEY(comm_bluetooth_service_id)
    REFERENCES comm_bluetooth_service (id)
      ON DELETE CASCADE
      ON UPDATE NO ACTION
);

CREATE UNIQUE INDEX comm_bluetooth_characteristic_endpoint_unique ON comm_bluetooth_characteristic(comm_bluetooth_service_id, endpoint);
CREATE UNIQUE INDEX comm_bluetooth_characteristic_uuid_unique ON comm_bluetooth_characteristic(comm_bluetooth_service_id, characteristic_uuid);

CREATE TABLE comm_xinput (
  protocol_id INTEGER PRIMARY KEY NOT NULL,
  FOREIGN KEY(protocol_id)
    REFERENCES protocol (id)
      ON DELETE CASCADE
      ON UPDATE NO ACTION
);

CREATE TABLE comm_hid (
  id INTEGER PRIMARY KEY NOT NULL,
  protocol_id INTEGER NOT NULL,
  hid_vendor_id INTEGER NOT NULL,
  hid_product_id INTEGER NOT NULL,
  FOREIGN KEY(protocol_id)
    REFERENCES protocol (id)
      ON DELETE CASCADE
      ON UPDATE NO ACTION
);

CREATE UNIQUE INDEX comm_hid_unique ON comm_hid(protocol_id, hid_vendor_id, hid_product_id);

CREATE TABLE comm_usb (
  id INTEGER PRIMARY KEY NOT NULL,
  protocol_id INTEGER NOT NULL,
  usb_vendor_id INTEGER NOT NULL,
  usb_product_id INTEGER NOT NULL,
  FOREIGN KEY(protocol_id)
    REFERENCES protocol (id)
      ON DELETE CASCADE
      ON UPDATE NO ACTION
);

CREATE UNIQUE INDEX comm_usb_unique ON comm_usb(protocol_id, usb_vendor_id, usb_product_id);

CREATE TABLE user_comm_serial (
  id INTEGER PRIMARY KEY NOT NULL,
  protocol_id INTEGER NOT NULL,
  port TEXT UNIQUE NOT NULL,
  baud INTEGER NOT NULL,
  data_bits INTEGER NOT NULL,
  stop_bits INTEGER NOT NULL,
  parity TEXT NOT NULL,
  FOREIGN KEY(protocol_id)
    REFERENCES protocol (id)
      ON DELETE CASCADE
      ON UPDATE NO ACTION
);

CREATE UNIQUE INDEX user_comm_serial_unique ON user_comm_serial(protocol_id, port);

CREATE TABLE user_comm_websocket_name (
  id INTEGER PRIMARY KEY NOT NULL,
  protocol_id INTEGER NOT NULL,
  device_name TEXT NOT NULL,
  FOREIGN KEY(protocol_id)
    REFERENCES protocol (id)
      ON DELETE CASCADE
      ON UPDATE NO ACTION
);

CREATE UNIQUE INDEX user_comm_websocket_name_unique ON user_comm_websocket_name(protocol_id, device_name);

CREATE TABLE user_comm_websocket_prefix (
  id INTEGER PRIMARY KEY NOT NULL,
  protocol_id INTEGER NOT NULL,
  device_prefix TEXT NOT NULL,
  FOREIGN KEY(protocol_id)
    REFERENCES protocol (id)
      ON DELETE CASCADE
      ON UPDATE NO ACTION
);

CREATE UNIQUE INDEX user_comm_websocket_prefix_unique ON user_comm_websocket_prefix(protocol_id, device_prefix);

CREATE TABLE device (
  id INTEGER PRIMARY KEY NOT NULL,
  protocol_id INTEGER NOT NULL,
  identifier TEXT,
  device_name TEXT NOT NULL,
  FOREIGN KEY(protocol_id)
    REFERENCES protocol (id)
      ON DELETE CASCADE
      ON UPDATE NO ACTION
);

CREATE UNIQUE INDEX device_protocol_identifier_index ON device (protocol_id, identifier);

CREATE TABLE user_device (
  id INTEGER PRIMARY KEY NOT NULL,
  device_id INTEGER NOT NULL,
  display_name TEXT,
  device_address TEXT,
  allow INTEGER NOT NULL CHECK (allow IN (0, 1)),
  deny INTEGER NOT NULL CHECK (deny IN (0, 1)),
  FOREIGN KEY(device_id)
    REFERENCES device (id)
      ON DELETE CASCADE
      ON UPDATE NO ACTION
);

CREATE TABLE feature_type (
  id INTEGER PRIMARY KEY NOT NULL,
  typename TEXT NOT NULL
);

INSERT INTO feature_type (id, typename) VALUES (1, "Unknown");
INSERT INTO feature_type (id, typename) VALUES (2, "Vibrate");
INSERT INTO feature_type (id, typename) VALUES (3, "Rotate");
INSERT INTO feature_type (id, typename) VALUES (4, "Oscillate");
INSERT INTO feature_type (id, typename) VALUES (5, "Constrict");
INSERT INTO feature_type (id, typename) VALUES (6, "Inflate");
INSERT INTO feature_type (id, typename) VALUES (7, "Position");
INSERT INTO feature_type (id, typename) VALUES (8, "Battery");
INSERT INTO feature_type (id, typename) VALUES (9, "RSSI");
INSERT INTO feature_type (id, typename) VALUES (10, "Button");
INSERT INTO feature_type (id, typename) VALUES (11, "Pressure");

CREATE TABLE feature_message(
  id INTEGER PRIMARY KEY NOT NULL,
  message_name TEXT NOT NULL
);

INSERT INTO feature_message(id, message_name) VALUES(1, "ScalarCmd");
INSERT INTO feature_message(id, message_name) VALUES(2, "RotateWithDirectionCmd");
INSERT INTO feature_message(id, message_name) VALUES(3, "PositionWithDurationCmd");
INSERT INTO feature_message(id, message_name) VALUES(4, "ReadCmd");
INSERT INTO feature_message(id, message_name) VALUES(5, "SubscribeCmd");

CREATE TABLE device_feature (
  id INTEGER PRIMARY KEY NOT NULL,
  device_id INTEGER NOT NULL,
  feature_type_id INTEGER NOT NULL,
  descriptor TEXT,
  range_min INTEGER NOT NULL,
  range_max INTEGER NOT NULL,
  FOREIGN KEY(feature_type_id)
    REFERENCES feature_type (id)
      ON DELETE CASCADE
      ON UPDATE NO ACTION,
  FOREIGN KEY(device_id)
    REFERENCES device (id)
      ON DELETE CASCADE
      ON UPDATE NO ACTION
);

CREATE TABLE device_feature_message (
  feature_id INTEGER NOT NULL,
  feature_message_id INTEGER NOT NULL,
  PRIMARY KEY (feature_id, feature_message_id),
  FOREIGN KEY(feature_id)
    REFERENCES device_feature (id)
      ON DELETE CASCADE
      ON UPDATE NO ACTION,
  FOREIGN KEY(feature_message_id)
    REFERENCES feature_message (id)
      ON DELETE CASCADE
      ON UPDATE NO ACTION
);

-- CREATE UNIQUE INDEX device_feature_message_unique ON device_feature_message(feature_id, feature_message_id);

CREATE TABLE user_device_feature (
  id INTEGER PRIMARY KEY NOT NULL,
  user_device_id INTEGER NOT NULL,
  device_feature_id INTEGER NOT NULL,
  range_min INTEGER NOT NULL,
  range_max INTEGER NOT NULL,
  FOREIGN KEY(user_device_id)
    REFERENCES user_device (id)
      ON DELETE CASCADE
      ON UPDATE NO ACTION,
  FOREIGN KEY(device_feature_id)
    REFERENCES device_feature (id)
      ON DELETE CASCADE
      ON UPDATE NO ACTION
);
