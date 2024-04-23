use crate::transport::{ClientTransport, ClientTransportEvent};
use serde::{de::DeserializeOwned, Serialize};

pub enum ClientEvent<Msg> {
	MsgFromServer(Msg),
	FailedToReceiveMsg(std::io::Error),
	FailedToParseMsg(Box<bincode::ErrorKind>),
	ServerDisconnected,
}

pub struct Client {
	transport: Box<dyn ClientTransport>,
}

impl Client {
	pub fn new(stream_transport: Box<dyn ClientTransport>) -> Self {
		Self {
			transport: stream_transport,
		}
	}

	pub fn handle_event<Msg: DeserializeOwned>(&mut self) -> Option<ClientEvent<Msg>> {
		if let Some(event) = self.transport.receive_event() {
			match event {
				ClientTransportEvent::ServerDisconnected => Some(ClientEvent::ServerDisconnected),
				ClientTransportEvent::FailedToReceiveMsg(e) => Some(ClientEvent::FailedToReceiveMsg(e)),
				ClientTransportEvent::NewMsg(data) => {
					match bincode::deserialize(&data) {
						Ok(msg) => Some(ClientEvent::MsgFromServer(msg)),
						Err(e) => Some(ClientEvent::FailedToParseMsg(e)),
					}
				}
			}
		}
		else {
			None
		}
	}

	pub fn send<T: Serialize>(&mut self, msg: &T) {
		if let Ok(data) = bincode::serialize(msg) {
			self.transport.send(&data);
		}
	}
}
