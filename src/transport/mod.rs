use std::net::IpAddr;
use std::io;

// NOTE: Changing this is not recommended, since it
pub const MAX_MSG_SIZE: usize = 65507;

pub mod tcp;
pub mod udp;

pub struct TransportMsg {
	pub sender_address: IpAddr,
	pub data: Vec<u8>,
}

pub enum ServerTransportEvent {
	NewClient(IpAddr),
	ClientDisconnected(IpAddr),
	FailedToReceiveMsg(io::Error),
	NewMsg(TransportMsg),
	FailedToAcceptConnection(io::Error),
}

pub trait ServerTransport {
	fn receive_event(&mut self) -> Option<ServerTransportEvent>;
}

pub trait ClientTransport {
	fn send(&mut self, data: &[u8]) -> io::Result<()>;
}
