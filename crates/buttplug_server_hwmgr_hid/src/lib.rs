#[macro_use]
extern crate log;

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub mod hid_comm_manager;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub mod hid_device_impl;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
mod hidapi_async;

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub use hid_comm_manager::{HidCommunicationManager, HidCommunicationManagerBuilder};
