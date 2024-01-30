use std::net::{ToSocketAddrs, TcpStream};
use std::io::{self};
use std::sync::{Arc, Mutex};
use crate::{ClientId, RawMessageData, ServerPacket};
use crate::client_impl::SharedData;
use crate::internal::InternalClientPacket;

pub struct Client {
	shared_data: Arc<Mutex<SharedData>>,
}

impl Client {
	pub fn new(listen_stream: TcpStream, send_stream: TcpStream) -> Self {
		let shared_data = Arc::new(Mutex::new(SharedData::new(send_stream)));
		let shared_data_clone = shared_data.clone();
		std::thread::spawn(move || Self::listen_thread(listen_stream, shared_data_clone));

		Self {
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

	pub fn send_to(&mut self, client_to_send_to: ClientId, message: &RawMessageData) -> io::Result<()> {
		self.shared_data.lock().unwrap().send_packet(&InternalClientPacket::PersonalMessage(client_to_send_to, message.clone()))
	}
	pub fn send_to_all(&mut self, message: &RawMessageData) -> io::Result<()> {
		self.shared_data.lock().unwrap().send_packet(&InternalClientPacket::BroadcastMessage(message.clone()))
	}
	pub fn send_to_server(&mut self, message: &RawMessageData) -> io::Result<()> {
		self.shared_data.lock().unwrap().send_packet(&InternalClientPacket::ServerMessage(message.clone()))
	}

	pub fn get_packet(&mut self) -> Option<ServerPacket> {
		if let Ok(mut shared_data) = self.shared_data.lock() {
			shared_data.packets.pop_back()
		}
		else {
			None
		}
	}
}