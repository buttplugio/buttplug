// @generated automatically by Diesel CLI.

diesel::table! {
    comm_bluetooth_characteristic (id) {
        id -> Integer,
        comm_bluetooth_service_id -> Integer,
        endpoint -> Text,
        characteristic_uuid -> Text,
    }
}

diesel::table! {
    comm_bluetooth_manufacturer_data (id) {
        id -> Integer,
        protocol_id -> Integer,
        manufacturer_company -> Integer,
        manufacturer_data -> Binary,
    }
}

diesel::table! {
    comm_bluetooth_name (id) {
        id -> Integer,
        protocol_id -> Integer,
        bluetooth_name -> Text,
    }
}

diesel::table! {
    comm_bluetooth_prefix (id) {
        id -> Integer,
        protocol_id -> Integer,
        bluetooth_prefix -> Text,
    }
}

diesel::table! {
    comm_bluetooth_service (id) {
        id -> Integer,
        protocol_id -> Integer,
        service_uuid -> Text,
    }
}

diesel::table! {
    comm_hid (id) {
        id -> Integer,
        protocol_id -> Integer,
        hid_vendor_id -> Integer,
        hid_product_id -> Integer,
    }
}

diesel::table! {
    comm_usb (id) {
        id -> Integer,
        protocol_id -> Integer,
        usb_vendor_id -> Integer,
        usb_product_id -> Integer,
    }
}

diesel::table! {
    comm_xinput (protocol_id) {
        protocol_id -> Integer,
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
        feature_type_id -> Integer,
        descriptor -> Nullable<Text>,
        range_min -> Integer,
        range_max -> Integer,
    }
}

diesel::table! {
    device_feature_message (feature_id, feature_message_id) {
        feature_id -> Integer,
        feature_message_id -> Integer,
    }
}

diesel::table! {
    feature_message (id) {
        id -> Integer,
        message_name -> Text,
    }
}

diesel::table! {
    feature_type (id) {
        id -> Integer,
        typename -> Text,
    }
}

diesel::table! {
    protocol (id) {
        id -> Integer,
        protocol_name -> Text,
        protocol_display_name -> Text,
    }
}

diesel::table! {
    user_comm_serial (id) {
        id -> Integer,
        protocol_id -> Integer,
        port -> Text,
        baud -> Integer,
        data_bits -> Integer,
        stop_bits -> Integer,
        parity -> Text,
    }
}

diesel::table! {
    user_comm_websocket_name (id) {
        id -> Integer,
        protocol_id -> Integer,
        device_name -> Text,
    }
}

diesel::table! {
    user_comm_websocket_prefix (id) {
        id -> Integer,
        protocol_id -> Integer,
        device_prefix -> Text,
    }
}

diesel::table! {
    user_device (id) {
        id -> Integer,
        device_id -> Integer,
        display_name -> Nullable<Text>,
        device_address -> Nullable<Text>,
        allow -> Integer,
        deny -> Integer,
    }
}

diesel::table! {
    user_device_feature (id) {
        id -> Integer,
        user_device_id -> Integer,
        device_feature_id -> Integer,
        range_min -> Integer,
        range_max -> Integer,
    }
}

diesel::joinable!(comm_bluetooth_characteristic -> comm_bluetooth_service (comm_bluetooth_service_id));
diesel::joinable!(comm_bluetooth_manufacturer_data -> protocol (protocol_id));
diesel::joinable!(comm_bluetooth_name -> protocol (protocol_id));
diesel::joinable!(comm_bluetooth_prefix -> protocol (protocol_id));
diesel::joinable!(comm_bluetooth_service -> protocol (protocol_id));
diesel::joinable!(comm_hid -> protocol (protocol_id));
diesel::joinable!(comm_usb -> protocol (protocol_id));
diesel::joinable!(comm_xinput -> protocol (protocol_id));
diesel::joinable!(device -> protocol (protocol_id));
diesel::joinable!(device_feature -> device (device_id));
diesel::joinable!(device_feature -> feature_type (feature_type_id));
diesel::joinable!(device_feature_message -> device_feature (feature_id));
diesel::joinable!(device_feature_message -> feature_message (feature_message_id));
diesel::joinable!(user_comm_serial -> protocol (protocol_id));
diesel::joinable!(user_comm_websocket_name -> protocol (protocol_id));
diesel::joinable!(user_comm_websocket_prefix -> protocol (protocol_id));
diesel::joinable!(user_device -> device (device_id));
diesel::joinable!(user_device_feature -> device_feature (device_feature_id));
diesel::joinable!(user_device_feature -> user_device (user_device_id));

diesel::allow_tables_to_appear_in_same_query!(
    comm_bluetooth_characteristic,
    comm_bluetooth_manufacturer_data,
    comm_bluetooth_name,
    comm_bluetooth_prefix,
    comm_bluetooth_service,
    comm_hid,
    comm_usb,
    comm_xinput,
    device,
    device_feature,
    device_feature_message,
    feature_message,
    feature_type,
    protocol,
    user_comm_serial,
    user_comm_websocket_name,
    user_comm_websocket_prefix,
    user_device,
    user_device_feature,
);
