-- This file should undo anything in `up.sql`

DROP TABLE protocol_bluetooth_name;
DROP TABLE protocol_bluetooth_manufacturer_data;
DROP TABLE protocol_bluetooth_characteristic;
DROP TABLE protocol_bluetooth_service;
DROP TABLE protocol_xinput;
DROP TABLE protocol_serial;
DROP TABLE protocol_hid;
DROP TABLE protocol_usb;
DROP TABLE user_protocol_serial;
DROP TABLE user_protocol_websocket_name;
DROP TABLE user_protocol_websocket_prefix;
DROP TABLE feature_linearcmd;
DROP TABLE feature_rotatecmd;
DROP TABLE feature_scalarcmd;
DROP TABLE feature_sensorcmd;
DROP TABLE device_feature;
DROP TABLE device;
DROP TABLE protocol;
DROP TABLE actuator_type;
DROP TABLE sensor_type;
