// @generated automatically by Diesel CLI.

diesel::table! {
    actuator_type (id) {
        id -> Integer,
        typename -> Text,
    }
}

diesel::table! {
    device (id) {
        id -> Integer,
        protocol_id -> Integer,
        identifier -> Nullable<Text>,
        device_name -> Text,
    }
}

diesel::table! {
    device_feature (id) {
        id -> Integer,
        device_id -> Integer,
        descriptor -> Nullable<Text>,
    }
}

diesel::table! {
    feature_linearcmd (id) {
        id -> Integer,
        feature_id -> Integer,
        description -> Nullable<Text>,
        range_min -> Integer,
        range_max -> Integer,
    }
}

diesel::table! {
    feature_rotatecmd (id) {
        id -> Integer,
        feature_id -> Integer,
        description -> Text,
        range_min -> Integer,
        range_max -> Integer,
    }
}

diesel::table! {
    feature_scalarcmd (id) {
        id -> Integer,
        feature_id -> Integer,
        actuator_type_id -> Integer,
        description -> Nullable<Text>,
        range_min -> Integer,
        range_max -> Integer,
    }
}

diesel::table! {
    feature_sensorcmd (id) {
        id -> Integer,
        feature_id -> Integer,
        sensor_type_id -> Integer,
        description -> Nullable<Text>,
        range_min -> Integer,
        range_max -> Integer,
        readable -> Integer,
        writable -> Integer,
    }
}

diesel::table! {
    protocol (id) {
        id -> Integer,
        protocol_name -> Text,
        display_name -> Text,
    }
}

diesel::table! {
    protocol_bluetooth_characteristic (id) {
        id -> Integer,
        protocol_bluetooth_service_id -> Integer,
        endpoint -> Text,
        characteristic_uuid -> Text,
    }
}

diesel::table! {
    protocol_bluetooth_manufacturer_data (id) {
        id -> Integer,
        protocol_id -> Integer,
        manufacturer_company -> Integer,
        manufacturer_data -> Binary,
    }
}

diesel::table! {
    protocol_bluetooth_name (id) {
        id -> Integer,
        protocol_id -> Integer,
        bluetooth_name -> Text,
        prefix -> Integer,
    }
}

diesel::table! {
    protocol_bluetooth_prefix (id) {
        id -> Integer,
        protocol_id -> Integer,
        prefix -> Text,
    }
}

diesel::table! {
    protocol_bluetooth_service (id) {
        id -> Integer,
        protocol_id -> Integer,
        service_uuid -> Text,
    }
}

diesel::table! {
    protocol_hid (protocol_id, hid_vendor_id, hid_product_id) {
        protocol_id -> Integer,
        hid_vendor_id -> Integer,
        hid_product_id -> Integer,
    }
}

diesel::table! {
    protocol_serial (protocol_id) {
        protocol_id -> Integer,
    }
}

diesel::table! {
    protocol_usb (protocol_id, usb_vendor_id, usb_product_id) {
        protocol_id -> Integer,
        usb_vendor_id -> Integer,
        usb_product_id -> Integer,
    }
}

diesel::table! {
    protocol_xinput (protocol_id) {
        protocol_id -> Integer,
    }
}

diesel::table! {
    sensor_type (id) {
        id -> Integer,
        typename -> Text,
    }
}

diesel::table! {
    user_protocol_serial (protocol_id, port) {
        protocol_id -> Integer,
        port -> Text,
        baud -> Integer,
        data_bits -> Integer,
        stop_bits -> Integer,
        parity -> Text,
    }
}

diesel::table! {
    user_protocol_websocket_name (id) {
        id -> Integer,
        protocol_id -> Integer,
        device_name -> Text,
    }
}

diesel::table! {
    user_protocol_websocket_prefix (id) {
        id -> Integer,
        protocol_id -> Integer,
        device_name -> Text,
    }
}

diesel::joinable!(device -> protocol (protocol_id));
diesel::joinable!(device_feature -> device (device_id));
diesel::joinable!(feature_linearcmd -> device_feature (feature_id));
diesel::joinable!(feature_rotatecmd -> device_feature (feature_id));
diesel::joinable!(feature_scalarcmd -> device_feature (feature_id));
diesel::joinable!(feature_sensorcmd -> device_feature (feature_id));
diesel::joinable!(feature_sensorcmd -> sensor_type (sensor_type_id));
diesel::joinable!(protocol_bluetooth_characteristic -> protocol_bluetooth_service (protocol_bluetooth_service_id));
diesel::joinable!(protocol_bluetooth_manufacturer_data -> protocol (protocol_id));
diesel::joinable!(protocol_bluetooth_name -> protocol (protocol_id));
diesel::joinable!(protocol_bluetooth_prefix -> protocol (protocol_id));
diesel::joinable!(protocol_bluetooth_service -> protocol (protocol_id));
diesel::joinable!(protocol_hid -> protocol (protocol_id));
diesel::joinable!(protocol_serial -> protocol (protocol_id));
diesel::joinable!(protocol_usb -> protocol (protocol_id));
diesel::joinable!(protocol_xinput -> protocol (protocol_id));
diesel::joinable!(user_protocol_serial -> protocol (protocol_id));
diesel::joinable!(user_protocol_websocket_name -> protocol (protocol_id));
diesel::joinable!(user_protocol_websocket_prefix -> protocol (protocol_id));

diesel::allow_tables_to_appear_in_same_query!(
    actuator_type,
    device,
    device_feature,
    feature_linearcmd,
    feature_rotatecmd,
    feature_scalarcmd,
    feature_sensorcmd,
    protocol,
    protocol_bluetooth_characteristic,
    protocol_bluetooth_manufacturer_data,
    protocol_bluetooth_name,
    protocol_bluetooth_prefix,
    protocol_bluetooth_service,
    protocol_hid,
    protocol_serial,
    protocol_usb,
    protocol_xinput,
    sensor_type,
    user_protocol_serial,
    user_protocol_websocket_name,
    user_protocol_websocket_prefix,
);
