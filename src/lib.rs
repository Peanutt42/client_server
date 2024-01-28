pub mod client;
pub use client::Client;
pub mod server;
pub use server::Server;

mod internal;

pub const MAX_MESSAGE_SIZE: usize = 1024;

pub type ClientId = usize;
pub type RawMessageData = Vec<u8>;

pub enum ClientMessage {
	Connected,
	ClientToClientMessage(RawMessageData),
	ClientToServerMessage(RawMessageData),
	Disconnected,
}

pub struct ClientPacket {
	pub author: ClientId,
	pub message: ClientMessage,
}

impl ClientPacket {
	pub fn new(author: ClientId, message: ClientMessage) -> Self {
		Self {
            author,
            message,
        }
	}
}

pub enum ServerPacket {
	ConnectedSuccessfully,
	NewClientConnected(ClientId),
	ClientDisconnected(ClientId),
	ClientKicked(ClientId),
	YouWereKicked,
	/// author, message
	ClientToClientMessage(ClientId, RawMessageData),
	ServerToClientMessage(RawMessageData),
	Disconnected,
	ConnectionRefused,
}