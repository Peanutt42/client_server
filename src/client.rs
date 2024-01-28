use std::collections::VecDeque;
use std::net::{ToSocketAddrs, TcpStream};
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use crate::{ClientId, RawMessageData, ServerPacket, MAX_MESSAGE_SIZE};
use crate::internal::{InternalClientPacket, InternalServerPacket};

struct SharedData {
	packets: VecDeque<ServerPacket>,
	id: Option<ClientId>,
}

impl SharedData {
	fn new() -> Self {
		Self {
			packets: VecDeque::new(),
			id: None,
		}
	}
}

pub struct Client {
	send_stream: TcpStream,
	shared_data: Arc<Mutex<SharedData>>,
}

impl Client {
	pub fn new(listen_stream: TcpStream, send_stream: TcpStream) -> Self {
		let shared_data = Arc::new(Mutex::new(SharedData::new()));
		let shared_data_clone = shared_data.clone();
		std::thread::spawn(move || Self::listen_thread(listen_stream, shared_data_clone));

		Self {
			send_stream,
			shared_data,
		}
	}

	pub fn connect<A>(ip_address: A) -> io::Result<Self>
		where A: ToSocketAddrs
	{
		let listen_stream = TcpStream::connect(ip_address)?;
		let send_stream = listen_stream.try_clone()?;
		Ok(Self::new(listen_stream, send_stream))
	}

	pub fn connect_localhost(port: u16) -> io::Result<Self> {
		Self::connect(("127.0.0.1", port))
	}

	fn send_packet(&mut self, packet: &InternalClientPacket) -> io::Result<()> {
		match bincode::serialize(&packet) {
			Ok(buffer) => self.send_stream.write_all(buffer.as_slice()),
			Err(e) => Err(io::Error::new(io::ErrorKind::Other, e.to_string())),
		}
	}

	pub fn send_to(&mut self, client_to_send_to: ClientId, message: &RawMessageData) -> io::Result<()> {
		self.send_packet(&InternalClientPacket::PersonalMessage(client_to_send_to, message.clone()))
	}
	pub fn send_to_all(&mut self, message: &RawMessageData) -> io::Result<()> {
		self.send_packet(&InternalClientPacket::BroadcastMessage(message.clone()))
	}
	pub fn send_to_server(&mut self, message: &RawMessageData) -> io::Result<()> {
		self.send_packet(&InternalClientPacket::ServerMessage(message.clone()))
	}

	pub fn get_packet(&mut self) -> Option<ServerPacket> {
		if let Ok(mut shared_data) = self.shared_data.lock() {
			shared_data.packets.pop_back()
		}
		else {
			None
		}
	}

	fn listen_thread(mut stream: TcpStream, shared_data: Arc<Mutex<SharedData>>) {
		let mut buffer = [0; MAX_MESSAGE_SIZE];

		loop {
			match stream.read(&mut buffer) {
				Ok(bytes_read) => {
					if bytes_read == 0 {
						shared_data.lock().unwrap().packets.push_back(ServerPacket::Disconnected);
						break;
					}
	
					match bincode::deserialize(&buffer[..bytes_read]) {
						Ok(packet) => {
							let mut shared_data = shared_data.lock().unwrap();
							match packet {
								InternalServerPacket::ConnectResponse(client_id) => shared_data.id = Some(client_id),
								InternalServerPacket::NewClientConnected(client_id) => shared_data.packets.push_back(ServerPacket::NewClientConnected(client_id)),
								InternalServerPacket::ClientDisconnected(client_id) => shared_data.packets.push_back(ServerPacket::ClientDisconnected(client_id)),
								InternalServerPacket::ClientKicked(client_id) => shared_data.packets.push_back(ServerPacket::ClientKicked(client_id)),
								InternalServerPacket::YouWereKicked => shared_data.packets.push_back(ServerPacket::YouWereKicked),
								InternalServerPacket::ClientToClient(client_id, message) => shared_data.packets.push_back(ServerPacket::ClientToClientMessage(client_id, message)),
								InternalServerPacket::ServerToClient(message) => shared_data.packets.push_back(ServerPacket::ServerToClientMessage(message)),
							}
						},
						Err(e) => eprintln!("Failed to parse server packet: {e}"),
					}
				}
				Err(e) => {
					eprintln!("failed to read from server: {e}");
					break;
				}
			}
		}
	}
}