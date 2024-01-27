use std::collections::VecDeque;
use std::net::{ToSocketAddrs, TcpStream};
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};

pub struct ServerPacket {
	pub data: String,
}

impl ServerPacket {
	pub fn new(data: String) -> Self {
		Self {
			data,
		}
	}
}

pub struct Client {
	send_stream: TcpStream,
	packets: Arc<Mutex<VecDeque<ServerPacket>>>,
}

impl Client {
	pub fn new(listen_stream: TcpStream, send_stream: TcpStream) -> Self {
		let packets = Arc::new(Mutex::new(VecDeque::new()));
		let packets_clone = packets.clone();
		std::thread::spawn(move || Self::listen_thread(listen_stream, packets_clone));

		Self {
			send_stream,
			packets,
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

	pub fn send(&mut self, buf: &[u8]) -> io::Result<()> {
		self.send_stream.write_all(buf)
	}

	pub fn get_packet(&mut self) -> Option<ServerPacket> {
		if let Ok(mut packets) = self.packets.lock() {
			packets.pop_back()
		}
		else {
			None
		}
	}

	fn listen_thread(mut stream: TcpStream, packets: Arc<Mutex<VecDeque<ServerPacket>>>) {
		let mut buffer = [0; 512];

		loop {
			match stream.read(&mut buffer) {
				Ok(bytes_read) => {
					if bytes_read == 0 {
						println!("Server disconnected");
						break;
					}
	
					let message = String::from_utf8(Vec::from(&buffer[..bytes_read])).unwrap();
					packets.lock().unwrap().push_back(ServerPacket::new(message));
				}
				Err(e) => {
					eprintln!("failed to read from server: {e}");
					break;
				}
			}
		}
	}
}