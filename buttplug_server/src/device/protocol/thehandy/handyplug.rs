//
// [
// {
// "Ok": {
// "Id": 1
// }
// }
// ]

#[derive(Clone, Eq, PartialEq, ::prost::Message)]
pub struct Ok {
    #[prost(uint32, tag="1")]
    pub id: u32,
}
//
// [
// {
// "Error": {
// "Id": 1,
// "ErrorMessage": "Server received invalid JSON.",
// "ErrorCode": 3
// }
// }
// ]

#[derive(Clone, Eq, PartialEq, ::prost::Message)]
pub struct Error {
    #[prost(uint32, tag="1")]
    pub id: u32,
    #[prost(string, tag="2")]
    pub error_message: ::prost::alloc::string::String,
    #[prost(int32, tag="3")]
    pub error_code: i32,
}
//
// [
// {
// "Ping": {
// "Id": 1
// }
// }
// ]

#[derive(Clone, Eq, PartialEq, ::prost::Message)]
pub struct Ping {
    #[prost(uint32, tag="1")]
    pub id: u32,
}
//
// [
// {
// "RequestServerInfo": {
// "Id": 1,
// "ClientName": "Test Client",
// "MessageVersion": 1
// }
// }
// ]

#[derive(Clone, Eq, PartialEq, ::prost::Message)]
pub struct RequestServerInfo {
    #[prost(uint32, tag="1")]
    pub id: u32,
    #[prost(string, tag="2")]
    pub client_name: ::prost::alloc::string::String,
    #[prost(uint32, tag="3")]
    pub message_version: u32,
}
//
// [
// {
// "ServerInfo": {
// "Id": 1,
// "ServerName": "Test Server",
// "MessageVersion": 1,
// "MaxPingTime": 100
// }
// }
// ]

#[derive(Clone, Eq, PartialEq, ::prost::Message)]
pub struct ServerInfo {
    #[prost(uint32, tag="1")]
    pub id: u32,
    #[prost(string, tag="2")]
    pub server_name: ::prost::alloc::string::String,
    #[prost(uint32, tag="3")]
    pub message_version: u32,
    #[prost(uint32, tag="4")]
    pub max_ping_time: u32,
}
//
// [
// {
// "LinearCmd": {
// "Id": 1,
// "DeviceIndex": 0,
// "Vectors": [
// {
// "Index": 0,
// "Duration": 500,
// "Position": 0.3
// },
// {
// "Index": 1,
// "Duration": 1000,
// "Position": 0.8
// }
// ]
// }
// }
// ]

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LinearCmd {
    #[prost(uint32, tag="1")]
    pub id: u32,
    #[prost(uint32, tag="2")]
    pub device_index: u32,
    #[prost(message, repeated, tag="3")]
    pub vectors: ::prost::alloc::vec::Vec<linear_cmd::Vector>,
}
/// Nested message and enum types in `LinearCmd`.
pub mod linear_cmd {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Vector {
        #[prost(uint32, tag="1")]
        pub index: u32,
        #[prost(uint32, tag="2")]
        pub duration: u32,
        #[prost(double, tag="3")]
        pub position: f64,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Message {
    #[prost(oneof="message::Message", tags="100, 101, 102, 200, 201, 403")]
    pub message: ::core::option::Option<message::Message>,
}
/// Nested message and enum types in `Message`.
pub mod message {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Message {
        ///  Status Messages 
        #[prost(message, tag="100")]
        Ok(super::Ok),
        #[prost(message, tag="101")]
        Error(super::Error),
        #[prost(message, tag="102")]
        Ping(super::Ping),
        ///  Handshake Messages 
        #[prost(message, tag="200")]
        RequestServerInfo(super::RequestServerInfo),
        #[prost(message, tag="201")]
        ServerInfo(super::ServerInfo),
        ///  Generic Device Messages 
        #[prost(message, tag="403")]
        LinearCmd(super::LinearCmd),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Payload {
    #[prost(message, repeated, tag="1")]
    pub messages: ::prost::alloc::vec::Vec<Message>,
}
