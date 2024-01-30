use custom_error::custom_error;
use std::net::{TcpListener, ToSocketAddrs};
use std::io::{self};
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};
use crate::{ClientId, RawMessageData, ClientPacket};
use crate::server_impl::SharedData;
use crate::internal::InternalServerPacket;


custom_error! { pub Error
	SendError{ client_id: ClientId, io_error: io::Error } = "failed to send packet to client {client_id}: {io_error}",
	ReadError{ client_id: ClientId, io_error: io::Error } = "failed to read packet from client {client_id}: {io_error}",
	SerializePacket{ bincode_error: String } = "failed to serialize packet: {bincode_error}",
	DeserializePacket{ bincode_error: String } = "failed to deserialize packet: {bincode_error}",
	InvalidClientId{ client_id: ClientId } = "invalid client: {client_id}",
	NoPingIntervalSet = "update_ping() was called but no ping interval was set",
	ErrorAcceptingConnection{ io_error: io::Error } = "failed to accept connection: {io_error}",
}

pub type Result<T> = std::result::Result<T, Error>;

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

	pub fn broadcast_all(&mut self, message: &RawMessageData) -> Result<()> {
		self.shared_data.lock().unwrap().broadcast_all(&InternalServerPacket::ServerToClient(message.clone()))
	}

	/// if you want to broadcast to all clients, use `broadcast_all`
	pub fn broadcast(&mut self, message: &RawMessageData, ignored_client: ClientId) -> Result<()> {
		self.shared_data.lock().unwrap().broadcast(&InternalServerPacket::ServerToClient(message.clone()), ignored_client)
	}

	pub fn send(&mut self, message: &RawMessageData, client_id: ClientId) -> Result<()> {
		self.shared_data.lock().unwrap().send(&InternalServerPacket::ServerToClient(message.clone()), client_id)
	}

	pub fn update_ping(&mut self) -> Result<()> {
		let mut shared_data = self.shared_data.lock().unwrap();
		if shared_data.ping_interval.is_none() {
			return Err(Error::NoPingIntervalSet);
		}
		
		let ping_interval = shared_data.ping_interval.unwrap();
		let mut error = Ok(());
		for (client_id, client) in shared_data.clients.iter_mut() {
			let now = Instant::now();
			if (client.ping_send_time.is_some() && now.duration_since(client.ping_send_time.unwrap()) >= ping_interval) ||
				client.ping_send_time.is_none()
			{
				if let Err(e) = SharedData::stream_send(&InternalServerPacket::Ping, &mut client.stream, *client_id) {
					error = Err(e);
				}
				else {
					client.ping_send_time = Some(now);
				}
			}
		}
		error
	}

	pub fn get_ping(&self, client_id: ClientId) -> Option<Duration> {
		let shared_data = self.shared_data.lock().unwrap();
		let client = shared_data.clients.get(&client_id)?;
		client.last_ping
	}

	pub fn get_clients(&self) -> Vec<ClientId> {
		self.shared_data.lock().unwrap().clients.keys().copied().collect()
	}

	pub fn get_ping_interval(&self) -> Option<Duration> {
		self.shared_data.lock().unwrap().ping_interval
	}
	pub fn set_ping_interval(&mut self, interval: Duration) {
		self.shared_data.lock().unwrap().ping_interval = Some(interval);
	}
	pub fn disable_ping(&mut self) {
		self.shared_data.lock().unwrap().ping_interval = None;
	}

	/// updates the server
	/// if a ping_interval was set, it will get the ping every interval
	pub fn update(&mut self) -> Result<()> {
		if self.shared_data.lock().unwrap().ping_interval.is_some() {
			self.update_ping()?;
		}
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
			shared_data.clients.get(&client_id).map(|client| client.stream.peer_addr().unwrap().to_string())
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

	/// returns the error log
	/// Important: the error log is cleared after returning the "copy"
	pub fn get_error_log(&mut self) -> Vec<Error> {
		std::mem::take(&mut self.shared_data.lock().unwrap().error_log)
	}
}