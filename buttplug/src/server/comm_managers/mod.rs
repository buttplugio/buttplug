#[cfg(feature = "btleplug-manager")]
pub mod btleplug;
#[cfg(all(feature = "xinput", target_os = "windows"))]
pub mod xinput;