use crate::core::messages;
use crate::core::messages::ButtplugMessageUnion;

struct ButtplugServer {
    client_name: String,
    server_name: String,
    client_spec_version: u32
}

impl ButtplugServer {
    pub fn send_message(&self, msg: &ButtplugMessageUnion) -> ButtplugMessageUnion {
        match msg {
            ButtplugMessageUnion::StartScanning(_s) => return self.start_scanning(),
            ButtplugMessageUnion::StopScanning(_s) => return self.stop_scanning(),
            _ => return ButtplugMessageUnion::Ok(messages::Ok { id: 0 }),
        }
    }

    fn start_scanning(&self) -> ButtplugMessageUnion {
        ButtplugMessageUnion::Ok(messages::Ok { id: 0 })
    }

    fn stop_scanning(&self) -> ButtplugMessageUnion {
        ButtplugMessageUnion::Ok(messages::Ok { id: 0 })
    }
}


