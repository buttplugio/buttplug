use crate::core::{
    messages::{self,
               ButtplugMessage,
               MessageAttributes,
               ButtplugMessageUnion,
               VibrateCmd,
               DeviceAdded,
               DeviceMessageInfo,
               VibrateSubcommand},
    errors::{ButtplugError, ButtplugMessageError},
};
use futures::{Future, SinkExt, StreamExt, future::BoxFuture};
use std::collections::HashMap;
use futures_channel::mpsc;

// Send over both a message, and a channel to receive back our processing future on.
pub struct ButtplugClientDeviceMessage<'a> {
    msg: ButtplugMessageUnion,
    future_sender: mpsc::Sender<BoxFuture<'a, ButtplugMessageUnion>>
}

impl<'a> ButtplugClientDeviceMessage<'a> {
    pub fn new(msg: ButtplugMessageUnion,
               future_sender: mpsc::Sender<BoxFuture<'a, ButtplugMessageUnion>>)
               -> ButtplugClientDeviceMessage<'a> {
        ButtplugClientDeviceMessage {
            msg,
            future_sender
        }
    }
}

#[derive(Clone)]
pub struct ButtplugClientDevice<'a> {
    pub name: String,
    index: u32,
    allowed_messages: HashMap<String, MessageAttributes>,
    client_sender: mpsc::UnboundedSender<ButtplugClientDeviceMessage<'a>>,
}

impl<'a> ButtplugClientDevice<'a> {
    pub fn new(name: &str,
               index: u32,
               allowed_messages: HashMap<String, MessageAttributes>,
               client_sender: mpsc::UnboundedSender<ButtplugClientDeviceMessage<'a>>)
               -> ButtplugClientDevice<'a> {
        ButtplugClientDevice {
            name: name.to_owned(),
            index,
            allowed_messages,
            client_sender,
        }
    }

    async fn send_message(&mut self, msg: ButtplugMessageUnion) -> ButtplugMessageUnion {
        // We'll only ever use this channel for 1 message. Kinda sucks but eh.
        let (send, mut recv) = mpsc::channel(1);
        let id = msg.get_id();
        let out_msg = ButtplugClientDeviceMessage::new(msg, send);
        self.client_sender.send(out_msg).await;
        let maybe_fut = recv.next().await;
        if let Some(fut) = maybe_fut {
            fut.await
        } else {
            let mut err_msg = ButtplugMessageUnion::Error(messages::Error::new(messages::ErrorCode::ErrorUnknown, "Unknown rrror receiving return from device message."));
            err_msg.set_id(id);
            err_msg
        }
    }

    async fn send_message_expect_ok(&mut self, msg: ButtplugMessageUnion) -> Option<ButtplugError> {
        let msg = self.send_message(msg).await;
        match msg {
            ButtplugMessageUnion::Ok(_) => None,
            ButtplugMessageUnion::Error(_err) => Some(ButtplugError::from(_err)),
            _ => Some(ButtplugError::ButtplugMessageError(ButtplugMessageError { message: "Got unexpected message type.".to_owned() } )),
        }
    }

    pub async fn send_vibrate_cmd(&mut self, speed: f64) -> Option<ButtplugError> {
        self.send_message_expect_ok(ButtplugMessageUnion::VibrateCmd(VibrateCmd::new(vec!(VibrateSubcommand::new(0, speed))))).await
    }

    // pub async fn send_linear_cmd(&self) -> Option<ButtplugError> {
    //     None
    // }

    // pub async fn send_rotation_cmd(&self) -> Option<ButtplugError> {
    //     None
    // }
}

impl<'a> From<(&DeviceAdded, mpsc::UnboundedSender<ButtplugClientDeviceMessage<'a>>)> for ButtplugClientDevice<'a> {
    fn from(msg_sender_tuple: (&DeviceAdded, mpsc::UnboundedSender<ButtplugClientDeviceMessage<'a>>)) -> Self {
        let msg = msg_sender_tuple.0.clone();
        ButtplugClientDevice::new(&*msg.device_name, msg.device_index, msg.device_messages, msg_sender_tuple.1)
    }
}

impl<'a> From<(&DeviceMessageInfo, mpsc::UnboundedSender<ButtplugClientDeviceMessage<'a>>)> for ButtplugClientDevice<'a> {
    fn from(msg_sender_tuple: (&DeviceMessageInfo, mpsc::UnboundedSender<ButtplugClientDeviceMessage<'a>>)) -> Self {
        let msg = msg_sender_tuple.0.clone();
        ButtplugClientDevice::new(&*msg.device_name, msg.device_index, msg.device_messages, msg_sender_tuple.1)
    }
}
