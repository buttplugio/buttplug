mod serialport_comm_manager;
mod serialport_device_impl;

pub use serialport_comm_manager::{
  SerialPortCommunicationManager, SerialPortCommunicationManagerBuilder,
};
pub use serialport_device_impl::{SerialPortDeviceImpl, SerialPortDeviceImplCreator};
