use crate::server::{Server, ClientId};
use std::net::SocketAddr;
use std::hash::{Hash, Hasher};

impl Server {
	pub(crate) fn client_id_from_address(address: SocketAddr) -> ClientId {
		let mut hasher = std::hash::DefaultHasher::new();
		address.hash(&mut hasher);
		hasher.finish() as ClientId
	}
}
