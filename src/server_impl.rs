use crate::server::{Server, ClientId};
use std::net::IpAddr;
use std::hash::{Hash, Hasher};

impl Server {
	pub(crate) fn generate_new_client_id(address: IpAddr) -> ClientId {
		let mut hasher = std::hash::DefaultHasher::new();
		address.hash(&mut hasher);
		hasher.finish() as ClientId
	}

	pub(crate) fn get_client_id(&mut self, address: IpAddr) -> ClientId {
		self.address_to_client_id
			.entry(address)
			.or_insert(Self::generate_new_client_id(address))
			.clone()
	}
}
