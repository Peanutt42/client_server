use std::io;
use std::net::SocketAddr;
use std::collections::HashMap;
use serde::{de::DeserializeOwned, Serialize};
use crate::transport::{ServerTransport, ServerTransportEvent};

pub type ClientId = usize;

#[derive(Debug, Clone)]
pub struct ClientMsg<Msg> {
	pub sender_id: ClientId,
	pub msg: Msg,
}

pub enum ServerEvent<Msg> {
	NewClient(ClientId),
	ClientDisconnected(ClientId),
	NewMsg(ClientMsg<Msg>),
	FailedToParseMsg(ClientId),
	FailedToAcceptConnection(io::Error),
	FailedToReceiveMsg(io::Error),
}

pub struct Server {
	transport: Box<dyn ServerTransport>,
	pub(crate) client_addresses: HashMap<ClientId, SocketAddr>,
}

impl Server {
	pub fn new(transport: Box<dyn ServerTransport>) -> Self {
		Self {
			transport,
			client_addresses: HashMap::new(),
		}
	}

	pub fn send_to<Msg: Serialize>(&mut self, client_id: ClientId, msg: &Msg) {
		if let Some(address) = self.client_addresses.get(&client_id) {
			if let Ok(bytes) = bincode::serialize(msg) {
				self.transport.send(*address, &bytes);
			}
		}
	}

	pub fn receive_event<Msg: DeserializeOwned>(&mut self) -> Option<ServerEvent<Msg>> {
		match self.transport.receive_event()? {
			ServerTransportEvent::NewClient(address) => {
				let client_id = Self::client_id_from_address(address);
				self.client_addresses.insert(client_id, address);
				Some(ServerEvent::NewClient(client_id))
			},
			ServerTransportEvent::ClientDisconnected(address) => {
				let client_id = Self::client_id_from_address(address);
				self.client_addresses.remove(&client_id);
				Some(ServerEvent::ClientDisconnected(client_id))
			},
			ServerTransportEvent::NewMsg(transport_msg) => {
				let sender_id = Self::client_id_from_address(transport_msg.sender_address);

				match bincode::deserialize(&transport_msg.data) {
					Ok(msg) => {
						Some(ServerEvent::NewMsg(ClientMsg {
							sender_id,
							msg,
						}))
					},
					Err(_e) => Some(ServerEvent::FailedToParseMsg(sender_id)),
				}
			},
			ServerTransportEvent::FailedToReceiveMsg(error) => Some(ServerEvent::FailedToReceiveMsg(error)),
			ServerTransportEvent::FailedToAcceptConnection(error) => Some(ServerEvent::FailedToAcceptConnection(error)),
		}
	}
}
