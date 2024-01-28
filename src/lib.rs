use serde::{Serialize, Deserialize};

pub mod client;
pub use client::Client;
pub mod server;
pub use server::Server;

pub const MAX_MESSAGE_SIZE: usize = 1024;

pub type ClientId = usize;

#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
	/// isn't sent by the client, but added internally when client connects
	Connected,
	ClientToClientMessage(String),
	Disconnected,
}

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
pub enum ServerPacket {
	ConnectResponse(ClientId),
	ClientConnected(ClientId),
	ClientDisconnected(ClientId),
	ClientKicked(ClientId),
	YouWereKicked,
	/// author, message
	ClientToClientMessage(ClientId, String),
	ServerToClientMessage(String),
	/// isn't sent from the server, but added internally when connection ends
	Disconnected,
}