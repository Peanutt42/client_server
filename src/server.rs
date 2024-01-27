use std::collections::{HashMap, VecDeque};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use crate::ClientId;

pub struct ClientPacket {
	pub sender: ClientId,
	pub data: String,
}

impl ClientPacket {
	pub fn new(sender: ClientId, data: String) -> Self {
		Self {
			sender,
            data,
		}
	}
}
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

	fn broadcast_all(&mut self, buf: &[u8]) -> io::Result<()> {
		for (_client_id, client) in self.clients.iter_mut() {
			client.stream.write_all(buf)?;
		}

		Ok(())
	}

	/// if you want to broadcast to all clients, use `broadcast_all`
	fn broadcast(&mut self, buf: &[u8], ignored_client: ClientId) -> io::Result<()> {
		for (client_id, client) in self.clients.iter_mut() {
			if *client_id != ignored_client {
				client.stream.write_all(buf)?;
			}
		}

		Ok(())
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
		let mut buffer = [0; 512];

		let client_id = shared_data.lock().unwrap().add_client(stream.try_clone().expect("Failed to clone stream"));
	
		loop {
			match stream.read(&mut buffer) {
				Ok(bytes_read) => {
					if bytes_read == 0 {
						shared_data.lock().unwrap().packets.push_back(ClientPacket::new(client_id, "disconnected".to_string()));
						break;
					}
	
					let message = &buffer[..bytes_read];
					shared_data.lock().unwrap().packets.push_back(ClientPacket::new(client_id, String::from_utf8(Vec::from(message)).unwrap()));
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

	pub fn broadcast_all(&mut self, buf: &[u8]) -> Result<(), String> {
		self.shared_data.lock()
			.map_err(|e| e.to_string())?
			.broadcast_all(buf)
			.map_err(|e| e.to_string())?;
		Ok(())
	}

	/// if you want to broadcast to all clients, use `broadcast_all`
	pub fn broadcast(&mut self, buf: &[u8], ignored_client: ClientId) -> Result<(), String> {
		self.shared_data.lock()
			.map_err(|e| e.to_string())?
			.broadcast(buf, ignored_client)
			.map_err(|e| e.to_string())?;
		Ok(())
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
}