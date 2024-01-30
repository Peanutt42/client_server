use std::net::TcpStream;
use std::collections::VecDeque;
use std::io::{self, Write, Read};
use std::sync::{Arc, Mutex};
use crate::{MAX_MESSAGE_SIZE, ClientId, ServerPacket};
use crate::internal::{InternalClientPacket, InternalServerPacket};
use crate::client::Client;


pub struct SharedData {
	pub send_stream: TcpStream,
	pub packets: VecDeque<ServerPacket>,
	pub id: Option<ClientId>,
}

impl SharedData {
	pub fn new(send_stream: TcpStream) -> Self {
		Self {
			send_stream,
			packets: VecDeque::new(),
			id: None,
		}
	}

	pub fn server_disconnected(&mut self) {
		self.packets.push_back(ServerPacket::Disconnected);
	}

	pub fn server_refused(&mut self) {
		self.packets.push_back(ServerPacket::ConnectionRefused);
	}

	pub fn send_packet(&mut self, packet: &InternalClientPacket) -> io::Result<()> {
		match bincode::serialize(&packet) {
			Ok(buffer) => self.send_stream.write_all(buffer.as_slice()),
			Err(e) => Err(io::Error::new(io::ErrorKind::Other, e.to_string())),
		}
	}
}

impl Client {
	pub fn listen_thread(mut stream: TcpStream, shared_data: Arc<Mutex<SharedData>>) {
		let mut buffer = [0; MAX_MESSAGE_SIZE];

		loop {
			match stream.read(&mut buffer) {
				Ok(bytes_read) => {
					if bytes_read == 0 {
						shared_data.lock().unwrap().server_disconnected();
						break;
					}
	
					match bincode::deserialize(&buffer[..bytes_read]) {
						Ok(packet) => {
							let mut shared_data = shared_data.lock().unwrap();
							match packet {
								InternalServerPacket::ConnectResponse(client_id) => {
									shared_data.id = Some(client_id);
									shared_data.packets.push_back(ServerPacket::ConnectedSuccessfully);
								},
								InternalServerPacket::NewClientConnected(client_id) => shared_data.packets.push_back(ServerPacket::NewClientConnected(client_id)),
								InternalServerPacket::ClientDisconnected(client_id) => shared_data.packets.push_back(ServerPacket::ClientDisconnected(client_id)),
								InternalServerPacket::ClientKicked(client_id) => shared_data.packets.push_back(ServerPacket::ClientKicked(client_id)),
								InternalServerPacket::YouWereKicked => shared_data.packets.push_back(ServerPacket::YouWereKicked),
								InternalServerPacket::ClientToClient(client_id, message) => shared_data.packets.push_back(ServerPacket::ClientToClientMessage(client_id, message)),
								InternalServerPacket::ServerToClient(message) => shared_data.packets.push_back(ServerPacket::ServerToClientMessage(message)),
								InternalServerPacket::Ping => {
									if let Err(e) = shared_data.send_packet(&InternalClientPacket::PingRespone) {
										eprintln!("error sending ping response: {e}");
									}
								},
							}
						},
						Err(e) => eprintln!("Failed to parse server packet: {e}"),
					}
				}
				Err(e) => {
					match e.kind() {
						io::ErrorKind::ConnectionAborted | io::ErrorKind::ConnectionReset => {
							shared_data.lock().unwrap().server_disconnected();
							break;
						},
						io::ErrorKind::ConnectionRefused => {
							shared_data.lock().unwrap().server_refused();
							break;
						}
						_ => eprintln!("failed to read from server: {e}"),
					}
					break;
				}
			}
		}
	}
}