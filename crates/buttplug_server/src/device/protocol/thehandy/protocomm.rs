///  Data structure of Session command/request packet 
#[derive(Clone, Eq, PartialEq, ::prost::Message)]
pub struct S0SessionCmd {
}
///  Data structure of Session response packet 
#[derive(Clone, Eq, PartialEq, ::prost::Message)]
pub struct S0SessionResp {
    #[prost(enumeration="Status", tag="1")]
    pub status: i32,
}
///  Payload structure of session data 
#[derive(Clone, Eq, PartialEq, ::prost::Message)]
pub struct Sec0Payload {
    /// !< Type of message 
    #[prost(enumeration="Sec0MsgType", tag="1")]
    pub msg: i32,
    #[prost(oneof="sec0_payload::Payload", tags="20, 21")]
    pub payload: ::core::option::Option<sec0_payload::Payload>,
}
/// Nested message and enum types in `Sec0Payload`.
pub mod sec0_payload {
    #[derive(Clone, Eq, PartialEq, ::prost::Oneof)]
    pub enum Payload {
        /// !< Payload data interpreted as Cmd 
        #[prost(message, tag="20")]
        Sc(super::S0SessionCmd),
        /// !< Payload data interpreted as Resp 
        #[prost(message, tag="21")]
        Sr(super::S0SessionResp),
    }
}
///  Data structure of Session command1 packet 
#[derive(Clone, Eq, PartialEq, ::prost::Message)]
pub struct SessionCmd1 {
    #[prost(bytes="vec", tag="2")]
    pub client_verify_data: ::prost::alloc::vec::Vec<u8>,
}
///  Data structure of Session response1 packet 
#[derive(Clone, Eq, PartialEq, ::prost::Message)]
pub struct SessionResp1 {
    #[prost(enumeration="Status", tag="1")]
    pub status: i32,
    #[prost(bytes="vec", tag="3")]
    pub device_verify_data: ::prost::alloc::vec::Vec<u8>,
}
///  Data structure of Session command0 packet 
#[derive(Clone, Eq, PartialEq, ::prost::Message)]
pub struct SessionCmd0 {
    #[prost(bytes="vec", tag="1")]
    pub client_pubkey: ::prost::alloc::vec::Vec<u8>,
}
///  Data structure of Session response0 packet 
#[derive(Clone, Eq, PartialEq, ::prost::Message)]
pub struct SessionResp0 {
    #[prost(enumeration="Status", tag="1")]
    pub status: i32,
    #[prost(bytes="vec", tag="2")]
    pub device_pubkey: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="3")]
    pub device_random: ::prost::alloc::vec::Vec<u8>,
}
///  Payload structure of session data 
#[derive(Clone, Eq, PartialEq, ::prost::Message)]
pub struct Sec1Payload {
    /// !< Type of message 
    #[prost(enumeration="Sec1MsgType", tag="1")]
    pub msg: i32,
    #[prost(oneof="sec1_payload::Payload", tags="20, 21, 22, 23")]
    pub payload: ::core::option::Option<sec1_payload::Payload>,
}
/// Nested message and enum types in `Sec1Payload`.
pub mod sec1_payload {
    #[derive(Clone, Eq, PartialEq, ::prost::Oneof)]
    pub enum Payload {
        /// !< Payload data interpreted as Cmd0 
        #[prost(message, tag="20")]
        Sc0(super::SessionCmd0),
        /// !< Payload data interpreted as Resp0 
        #[prost(message, tag="21")]
        Sr0(super::SessionResp0),
        /// !< Payload data interpreted as Cmd1 
        #[prost(message, tag="22")]
        Sc1(super::SessionCmd1),
        /// !< Payload data interpreted as Resp1 
        #[prost(message, tag="23")]
        Sr1(super::SessionResp1),
    }
}
///  Data structure exchanged when establishing
///  secure session between Host and Client 
#[derive(Clone, Eq, PartialEq, ::prost::Message)]
pub struct SessionData {
    /// !< Type of security 
    #[prost(enumeration="SecSchemeVersion", tag="2")]
    pub sec_ver: i32,
    #[prost(oneof="session_data::Proto", tags="10, 11")]
    pub proto: ::core::option::Option<session_data::Proto>,
}
/// Nested message and enum types in `SessionData`.
pub mod session_data {
    #[derive(Clone, Eq, PartialEq, ::prost::Oneof)]
    pub enum Proto {
        /// !< Payload data in case of security 0 
        #[prost(message, tag="10")]
        Sec0(super::Sec0Payload),
        /// !< Payload data in case of security 1 
        #[prost(message, tag="11")]
        Sec1(super::Sec1Payload),
    }
}
///  Allowed values for the status
///  of a protocomm instance 
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Status {
    Success = 0,
    InvalidSecScheme = 1,
    InvalidProto = 2,
    TooManySessions = 3,
    InvalidArgument = 4,
    InternalError = 5,
    CryptoError = 6,
    InvalidSession = 7,
}
impl Status {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Status::Success => "Success",
            Status::InvalidSecScheme => "InvalidSecScheme",
            Status::InvalidProto => "InvalidProto",
            Status::TooManySessions => "TooManySessions",
            Status::InvalidArgument => "InvalidArgument",
            Status::InternalError => "InternalError",
            Status::CryptoError => "CryptoError",
            Status::InvalidSession => "InvalidSession",
        }
    }
}
///  A message must be of type Cmd or Resp 
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Sec0MsgType {
    S0SessionCommand = 0,
    S0SessionResponse = 1,
}
impl Sec0MsgType {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Sec0MsgType::S0SessionCommand => "S0_Session_Command",
            Sec0MsgType::S0SessionResponse => "S0_Session_Response",
        }
    }
}
///  A message must be of type Cmd0 / Cmd1 / Resp0 / Resp1 
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Sec1MsgType {
    SessionCommand0 = 0,
    SessionResponse0 = 1,
    SessionCommand1 = 2,
    SessionResponse1 = 3,
}
impl Sec1MsgType {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Sec1MsgType::SessionCommand0 => "Session_Command0",
            Sec1MsgType::SessionResponse0 => "Session_Response0",
            Sec1MsgType::SessionCommand1 => "Session_Command1",
            Sec1MsgType::SessionResponse1 => "Session_Response1",
        }
    }
}
///  Allowed values for the type of security
///  being used in a protocomm session 
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum SecSchemeVersion {
    /// !< Unsecured - plaintext communication 
    SecScheme0 = 0,
    /// !< Security scheme 1 - Curve25519 + AES-256-CTR
    SecScheme1 = 1,
}
impl SecSchemeVersion {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            SecSchemeVersion::SecScheme0 => "SecScheme0",
            SecSchemeVersion::SecScheme1 => "SecScheme1",
        }
    }
}
