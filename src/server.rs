use std::io;
use std::net::IpAddr;
use std::collections::HashMap;
use serde::de::DeserializeOwned;
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
	pub(crate) address_to_client_id: HashMap<IpAddr, ClientId>,
}

impl Server {
	pub fn new(transport: Box<dyn ServerTransport>) -> Self {
		Self {
			transport,
			address_to_client_id: HashMap::new(),
		}
	}

	pub fn receive_event<Msg: DeserializeOwned>(&mut self) -> Option<ServerEvent<Msg>> {
		match self.transport.receive_event()? {
			ServerTransportEvent::NewClient(address) => {
				Some(ServerEvent::NewClient(self.get_client_id(address)))
			},
			ServerTransportEvent::ClientDisconnected(address) => {
				Some(ServerEvent::ClientDisconnected(self.get_client_id(address)))
				// TODO: Actually remove client info
			},
			ServerTransportEvent::NewMsg(transport_msg) => {
				let sender_id = self.get_client_id(transport_msg.sender_address);

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
