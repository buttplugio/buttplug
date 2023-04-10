// @generated automatically by Diesel CLI.

diesel::table! {
    actuator_type (id) {
        id -> Nullable<Integer>,
        actuator_type -> Nullable<Text>,
    }
}

diesel::table! {
    device (id) {
        id -> Nullable<Integer>,
        protocol_id -> Nullable<Integer>,
        identifier -> Nullable<Text>,
        device_name -> Text,
    }
}

diesel::table! {
    device_feature (id) {
        id -> Nullable<Integer>,
        device_id -> Nullable<Integer>,
        descriptor -> Nullable<Text>,
    }
}

diesel::table! {
    feature_linearcmd (id) {
        id -> Nullable<Integer>,
        feature_id -> Nullable<Integer>,
        description -> Nullable<Text>,
        range_min -> Nullable<Integer>,
        range_max -> Nullable<Integer>,
    }
}

diesel::table! {
    feature_rotatecmd (id) {
        id -> Nullable<Integer>,
        feature_id -> Nullable<Integer>,
        description -> Nullable<Text>,
        range_min -> Nullable<Integer>,
        range_max -> Nullable<Integer>,
    }
}

diesel::table! {
    feature_scalarcmd (id) {
        id -> Nullable<Integer>,
        feature_id -> Nullable<Integer>,
        actuator_type_id -> Nullable<Integer>,
        description -> Nullable<Text>,
        range_min -> Nullable<Integer>,
        range_max -> Nullable<Integer>,
    }
}

diesel::table! {
    feature_sensorcmd (id) {
        id -> Nullable<Integer>,
        feature_id -> Nullable<Integer>,
        sensor_type_id -> Nullable<Integer>,
        escription -> Nullable<Text>,
        range_min -> Nullable<Integer>,
        range_max -> Nullable<Integer>,
        readable -> Nullable<Integer>,
        writable -> Nullable<Integer>,
    }
}

diesel::table! {
    protocol (id) {
        id -> Nullable<Integer>,
        protocol_name -> Text,
        display_name -> Text,
    }
}

diesel::table! {
    protocol_bluetooth_characteristic (id) {
        id -> Nullable<Integer>,
        service_id -> Nullable<Integer>,
        endpoint -> Nullable<Text>,
        characteristic_uuid -> Nullable<Text>,
    }
}

diesel::table! {
    protocol_bluetooth_manufacturer_data (id) {
        id -> Nullable<Integer>,
        protocol_id -> Nullable<Integer>,
        manufacturer_company -> Nullable<Integer>,
        manufacturer_data -> Nullable<Binary>,
    }
}

diesel::table! {
    protocol_bluetooth_name (id) {
        id -> Nullable<Integer>,
        protocol_id -> Nullable<Integer>,
        bluetooth_name -> Nullable<Text>,
    }
}

diesel::table! {
    protocol_bluetooth_prefix (id) {
        id -> Nullable<Integer>,
        protocol_id -> Nullable<Integer>,
        prefix -> Nullable<Text>,
    }
}

diesel::table! {
    protocol_bluetooth_service (id) {
        id -> Nullable<Integer>,
        protocol_id -> Nullable<Integer>,
        service_uuid -> Nullable<Text>,
    }
}

diesel::table! {
    protocol_hid (protocol_id, hid_vendor_id, hid_product_id) {
        protocol_id -> Nullable<Integer>,
        hid_vendor_id -> Nullable<Integer>,
        hid_product_id -> Nullable<Integer>,
    }
}

diesel::table! {
    protocol_serial (protocol_id) {
        protocol_id -> Nullable<Integer>,
    }
}

diesel::table! {
    protocol_usb (protocol_id, usb_vendor_id, usb_product_id) {
        protocol_id -> Nullable<Integer>,
        usb_vendor_id -> Nullable<Integer>,
        usb_product_id -> Nullable<Integer>,
    }
}

diesel::table! {
    protocol_xinput (protocol_id) {
        protocol_id -> Nullable<Integer>,
    }
}

diesel::table! {
    sensor_type (id) {
        id -> Nullable<Integer>,
        sensor_type -> Nullable<Text>,
    }
}

diesel::table! {
    user_protocol_serial (protocol_id, port) {
        protocol_id -> Nullable<Integer>,
        port -> Nullable<Text>,
        baud -> Nullable<Integer>,
        data_bits -> Nullable<Integer>,
        stop_bits -> Nullable<Integer>,
        parity -> Nullable<Text>,
    }
}

diesel::table! {
    user_protocol_websocket_name (id) {
        id -> Nullable<Integer>,
        protocol_id -> Nullable<Integer>,
        device_name -> Nullable<Text>,
    }
}

diesel::table! {
    user_protocol_websocket_prefix (id) {
        id -> Nullable<Integer>,
        protocol_id -> Nullable<Integer>,
        device_name -> Nullable<Text>,
    }
}

diesel::joinable!(device -> protocol (protocol_id));
diesel::joinable!(device_feature -> device (device_id));
diesel::joinable!(feature_linearcmd -> device_feature (feature_id));
diesel::joinable!(feature_rotatecmd -> device_feature (feature_id));
diesel::joinable!(feature_scalarcmd -> device_feature (feature_id));
diesel::joinable!(feature_sensorcmd -> device_feature (feature_id));
diesel::joinable!(feature_sensorcmd -> sensor_type (sensor_type_id));
diesel::joinable!(protocol_bluetooth_characteristic -> protocol_bluetooth_service (service_id));
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
