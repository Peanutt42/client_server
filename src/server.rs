use std::collections::{HashMap, VecDeque};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use crate::{ClientId, RawMessageData, MAX_MESSAGE_SIZE, ClientPacket, ClientMessage};
use crate::internal::{InternalClientPacket, InternalServerPacket};

pub struct ClientData {
	stream: TcpStream,
}

impl ClientData {
	fn new(stream: TcpStream) -> Self {
		Self {
			stream,
		}
	}
}

pub struct SharedData {
	clients: HashMap<ClientId, ClientData>,
	next_id: ClientId,

	pub packets: VecDeque<ClientPacket>,
}

impl SharedData {
	fn new() -> Self {
		Self {
			clients: HashMap::new(),
			next_id: 0,
			packets: VecDeque::new(),
		}
	}

	fn add_client(&mut self, stream: TcpStream) -> ClientId {
		let client_id = self.next_id;
		self.next_id += 1;

		self.clients.insert(client_id, ClientData::new(stream));

		client_id
	}

	fn remove_client(&mut self, client_id: ClientId) {
		self.clients.remove(&client_id);
	}

	fn send(&mut self, packet: &InternalServerPacket, client_id: ClientId) -> io::Result<()> {
		match bincode::serialize(packet) {
			Ok(buffer) => {
				match self.clients.get_mut(&client_id) {
					Some(client) => client.stream.write_all(buffer.as_slice()),
					None => Err(io::Error::new(io::ErrorKind::Other, "Client id doesn't exist")),
				}
			},
			Err(e) => Err(io::Error::new(io::ErrorKind::Other, e.to_string())),
		}
	}

	fn broadcast_all(&mut self, packet: &InternalServerPacket) -> io::Result<()> {
		match bincode::serialize(packet) {
			Ok(buffer) => {
				let mut error_msg = Ok(());
				for (_client_id, client) in self.clients.iter_mut() {
					if let Err(e) = client.stream.write_all(buffer.as_slice()) {
						error_msg = Err(e);
					}
				}
				error_msg
			},
			Err(e) => Err(io::Error::new(io::ErrorKind::Other, e.to_string())),
		}
	}

	/// if you want to broadcast to all clients, use `broadcast_all`
	fn broadcast(&mut self, packet: &InternalServerPacket, ignored_client: ClientId) -> io::Result<()> {
		match bincode::serialize(packet) {
			Ok(buffer) => {
				let mut error_msg = Ok(());
				for (client_id, client) in self.clients.iter_mut() {
					if *client_id != ignored_client {
						if let Err(e) = client.stream.write_all(buffer.as_slice()) {
							error_msg = Err(e);
						}
					}
				}
				error_msg
			},
			Err(e) => Err(io::Error::new(io::ErrorKind::Other, e.to_string())),
		}
	}
}

pub struct Server {
	shared_data: Arc<Mutex<SharedData>>,
}

impl Server {
	pub fn new(listener: TcpListener) -> Self {
		let shared_data = Arc::new(Mutex::new(SharedData::new()));
		
		let shared_data_clone = shared_data.clone();
		std::thread::spawn(move || Self::listen_thread(listener, shared_data_clone));

		Self {
			shared_data,
		}
	}

	pub fn bind<A>(addr: A) -> io::Result<Self> 
	where A: ToSocketAddrs
	{
		let listener = TcpListener::bind(addr)?;
		Ok(Self::new(listener))
	}

	pub fn bind_localhost(port: u16) -> io::Result<Self> {
		Self::bind(("127.0.0.1", port))
	}

	fn handle_client_thread(mut stream: TcpStream, shared_data: Arc<Mutex<SharedData>>) {
		let mut buffer = [0; MAX_MESSAGE_SIZE];
		let client_id: ClientId;

		{
			let mut shared_data = shared_data.lock().unwrap();
			client_id = shared_data.add_client(stream.try_clone().expect("Failed to clone stream"));
			shared_data.packets.push_front(ClientPacket::new(client_id, ClientMessage::Connected));
			shared_data.send(&InternalServerPacket::ConnectResponse(client_id), client_id).expect("failed to send the connect response to client");
			shared_data.broadcast(&InternalServerPacket::NewClientConnected(client_id), client_id)
				.expect("failed to send 'ClientConnected' to all other clients");
		}

		loop {
			match stream.read(&mut buffer) {
				Ok(bytes_read) => {
					if bytes_read == 0 {
						let mut shared_data = shared_data.lock().unwrap();
						shared_data.packets.push_back(ClientPacket::new(client_id, ClientMessage::Disconnected));
						shared_data.broadcast(&InternalServerPacket::ClientDisconnected(client_id), client_id).expect("failed to broadcast that a client has disconnected to all the other clients");
						break;
					}
	
					match bincode::deserialize(&buffer[..bytes_read]) {
						Ok(packet) => {
							let mut shared_data = shared_data.lock().unwrap();
							match packet {
								InternalClientPacket::BroadcastMessage(message) => {
									shared_data.packets.push_back(ClientPacket::new(client_id, ClientMessage::ClientToClientMessage(message.clone())));
									if let Err(e) = shared_data.broadcast(&InternalServerPacket::ClientToClient(client_id, message), client_id) {
										eprintln!("failed to broadcast a client message to the other clients: {e}");
									}
								},
								InternalClientPacket::PersonalMessage(target_client_id, message) => {
									shared_data.packets.push_back(ClientPacket::new(client_id, ClientMessage::ClientToClientMessage(message.clone())));
                                    if let Err(e) = shared_data.send(&InternalServerPacket::ClientToClient(client_id, message), target_client_id) {
										eprintln!("failed to redirect a personal client message to another client: {e}");
									}
								},
								InternalClientPacket::ServerMessage(message) => {
                                    shared_data.packets.push_back(ClientPacket::new(client_id, ClientMessage::ClientToServerMessage(message)));
                                },
							};
						},
						Err(e) => eprintln!("failed to deserialize packet from client {client_id}: {e}"),
					}
				}
				Err(e) => {
					eprintln!("failed to read stream from client {client_id}: {e}");
					break;
				}
			}
		}
	
		shared_data.lock().unwrap().remove_client(client_id);
	}

	fn listen_thread(listener: TcpListener, shared_data: Arc<Mutex<SharedData>>) {
		for stream in listener.incoming() {
			match stream {
				Ok(stream) => {
					let shared_data_clone = shared_data.clone();
					std::thread::spawn(move || Self::handle_client_thread(stream, shared_data_clone));
				},
				Err(e) => {
					eprintln!("error accepting connection: {e}");
				}
			}
		}
	}

	pub fn broadcast_all(&mut self, message: &RawMessageData) -> Result<(), String> {
		self.shared_data.lock()
			.map_err(|e| e.to_string())?
			.broadcast_all(&InternalServerPacket::ServerToClient(message.clone()))
			.map_err(|e| e.to_string())
	}

	/// if you want to broadcast to all clients, use `broadcast_all`
	pub fn broadcast(&mut self, message: &RawMessageData, ignored_client: ClientId) -> Result<(), String> {
		self.shared_data.lock()
			.map_err(|e| e.to_string())?
			.broadcast(&InternalServerPacket::ServerToClient(message.clone()), ignored_client)
			.map_err(|e| e.to_string())
	}

	pub fn get_packet(&mut self) -> Option<ClientPacket> {
		if let Ok(mut shared_data) = self.shared_data.lock() {
			shared_data.packets.pop_back()
		}
		else {
			None
		}
	}

	pub fn get_client_ip_address(&self, client_id: ClientId) -> Option<String> {
		if let Ok(shared_data) = self.shared_data.lock() {
			match shared_data.clients.get(&client_id) {
				Some(client) => Some(client.stream.peer_addr().unwrap().to_string()),
				None => None,
			}
		}
		else {
			None
		}
	}

	pub fn kick_client(&mut self, client_id: ClientId) {
		if let Ok(mut shared_data) = self.shared_data.lock() {
			let _ = shared_data.send(&InternalServerPacket::YouWereKicked, client_id);
			shared_data.broadcast(&InternalServerPacket::ClientKicked(client_id), client_id).expect("failed to send notification to all other clients that one client got kicked");
            shared_data.remove_client(client_id);
        }
	}
}