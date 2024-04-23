use std::net::SocketAddr;
use std::io;

// NOTE: Changing this is not recommended, since it
pub const MAX_MSG_SIZE: usize = 65507;

pub mod tcp;
pub mod udp;

pub struct TransportMsg {
	pub sender_address: SocketAddr,
	pub data: Vec<u8>,
}

pub enum ClientTransportEvent {
	NewMsg(Vec<u8>),
	FailedToReceiveMsg(io::Error),
	ServerDisconnected,
}

pub enum ServerTransportEvent {
	NewClient(SocketAddr),
	ClientDisconnected(SocketAddr),
	FailedToReceiveMsg(io::Error),
	NewMsg(TransportMsg),
	FailedToAcceptConnection(io::Error),
}

pub trait ServerTransport {
	fn receive_event(&mut self) -> Option<ServerTransportEvent>;
	fn send(&mut self, address: SocketAddr, data: &[u8]);
}

pub trait ClientTransport {
	fn receive_event(&mut self) -> Option<ClientTransportEvent>;
	fn send(&mut self, data: &[u8]);
}
