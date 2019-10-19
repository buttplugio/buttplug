use crate::core::errors::ButtplugError;

trait DeviceSubtypeManager {
    fn start_scanning() -> Result<(), ButtplugError>;
    fn stop_scanning() -> Result<(), ButtplugError>;
    fn is_scanning() -> bool;
}

struct DeviceManager {}
