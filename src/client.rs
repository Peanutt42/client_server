use std::collections::VecDeque;
use std::net::{ToSocketAddrs, TcpStream};
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use crate::{ClientId, ClientMessage, ClientPacket, ServerPacket};

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

	pub fn send(&mut self, message: ClientMessage) -> io::Result<()> {
		match self.shared_data.lock().unwrap().id {
			Some(my_client_id) => {
				let packet = ClientPacket::new(my_client_id, message);
				match bincode::serialize(&packet) {
					Ok(buffer) => self.send_stream.write_all(buffer.as_slice()),
					Err(e) => Err(io::Error::new(io::ErrorKind::Other, e.to_string())),
				}
			},
			None => Err(io::Error::new(io::ErrorKind::Other, "ClientId not set, we are not connected to the server!".to_string())),
		}
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
		let mut buffer = [0; 512];

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
							if let ServerPacket::ConnectResponse(client_id) = &packet {
								shared_data.id = Some(*client_id);
							}
							shared_data.packets.push_back(packet)
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