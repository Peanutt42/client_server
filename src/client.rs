use std::collections::VecDeque;

use crate::transport::ClientTransport;
use serde::Serialize;

pub enum ClientEvent {
	ServerDisconnected,
}

pub struct Client {
	transport: Box<dyn ClientTransport>,
	events: VecDeque<ClientEvent>,
}

impl Client {
	pub fn new(stream_transport: Box<dyn ClientTransport>) -> Self {
		Self {
			transport: stream_transport,
			events: VecDeque::new(),
		}
	}

	pub fn handle_event(&mut self) -> Option<ClientEvent> {
		self.events.pop_front()
	}

	pub fn send<T: Serialize>(&mut self, msg: T) -> anyhow::Result<()> {
		let data = bincode::serialize(&msg)?;
		if self.transport.send(&data).is_err() {
			// TODO: move this into the transport event handeling, once we start receiving messages from the server
			self.events.push_back(ClientEvent::ServerDisconnected);
		}
		Ok(())
	}
}
