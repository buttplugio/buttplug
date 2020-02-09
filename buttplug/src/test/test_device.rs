use crate::{
    device::{
        Endpoint,
        device::{DeviceImpl, DeviceImplCommand, DeviceReadCmd, DeviceWriteCmd, 
            DeviceSubscribeCmd, DeviceUnsubscribeCmd, ButtplugDeviceEvent, ButtplugDeviceImplCreator},
        configuration_manager::{DeviceSpecifier, ProtocolDefinition},
    },
    core::{
        errors::{ ButtplugError, ButtplugDeviceError },
        messages::{ RawReading },
    }
};
use std::collections::HashMap;
use async_std::{
    sync::{channel, Sender, Receiver}
};
use async_trait::async_trait;


pub struct TestDeviceImplCreator {
    specifier: DeviceSpecifier,
    device_impl: Option<Box<dyn DeviceImpl>>,
}

impl TestDeviceImplCreator {
    pub fn new(specifier: DeviceSpecifier, device_impl: Box<dyn DeviceImpl>) -> Self {
        Self {
            specifier,
            device_impl: Some(device_impl)
        }
    }
}

#[async_trait]
impl ButtplugDeviceImplCreator for TestDeviceImplCreator {
    fn get_specifier(&self) -> DeviceSpecifier {
        self.specifier.clone()
    }

    async fn try_create_device_impl(
        &mut self,
        protocol: ProtocolDefinition,
    ) -> Result<Box<dyn DeviceImpl>, ButtplugError> {
        // TODO Should probably figure out how to check for endpoints here.
        Ok(self.device_impl.take().unwrap())
    }
}

#[derive(Clone)]
pub struct TestDevice {
    name: String,
    endpoints: Vec<Endpoint>,
    address: String,
    pub endpoint_channels: HashMap<Endpoint, (Sender<DeviceImplCommand>, Receiver<DeviceImplCommand>)>,
    pub event_sender: Sender<ButtplugDeviceEvent>,
    pub event_receiver: Receiver<ButtplugDeviceEvent>,
}

impl TestDevice {
    pub fn new(name: &str, endpoints: Vec<Endpoint>) -> Self {
        let mut endpoint_channels = HashMap::new();
        for endpoint in &endpoints {
            let (sender, receiver) = channel(256);
            endpoint_channels.insert(endpoint.clone(), (sender, receiver));
        }
        let (event_sender, event_receiver) = channel(256);
        Self {
            name: name.to_string(),
            address: "".to_string(),
            endpoints,
            endpoint_channels,
            event_sender,
            event_receiver
        }
    }
}

#[async_trait]
impl DeviceImpl for TestDevice {
    fn name(&self) -> &str {
        &self.name
    }

    fn address(&self) -> &str {
        &self.address
    }

    fn connected(&self) -> bool {
        true
    }

    fn endpoints(&self) -> Vec<Endpoint> {
        self.endpoints.clone()
    }

    async fn disconnect(&self) {

    }

    fn box_clone(&self) -> Box<dyn DeviceImpl> {
        Box::new((*self).clone())
    }

    fn get_event_receiver(&self) -> Receiver<ButtplugDeviceEvent> {
        self.event_receiver.clone()
    }

    async fn read_value(&self, msg: DeviceReadCmd) -> Result<RawReading, ButtplugError> {
        Ok(RawReading::new(0, msg.endpoint, vec!()))
    }

    async fn write_value(&self, msg: DeviceWriteCmd) -> Result<(), ButtplugError> {
        match self.endpoint_channels.get(&msg.endpoint) {
            Some((sender, _)) => {
                sender.send(msg.into()).await;
                Ok(())
            },
            None => {
                Err(ButtplugDeviceError::new(&format!("Endpoint {} does not exist for {}", msg.endpoint, self.name)).into())
            }
        }
    }

    async fn subscribe(&self, msg: DeviceSubscribeCmd) -> Result<(), ButtplugError> {
        Ok(())
    }

    async fn unsubscribe(&self, msg: DeviceUnsubscribeCmd) -> Result<(), ButtplugError> {
        Ok(())
    }
}