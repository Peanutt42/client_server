use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::io::{self, Read, Write};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::transport::{MAX_MSG_SIZE, ClientTransport, ServerTransport, TransportMsg, ServerTransportEvent};

use super::ClientTransportEvent;

pub struct TcpClientTransport {
	stream: TcpStream,
	transport_msg_receiver: Receiver<ClientTransportEvent>,
	server_disconnected: bool,
}

impl TcpClientTransport {
	pub fn new<A: ToSocketAddrs>(server_address: A) -> io::Result<Self> {
		let stream = TcpStream::connect(server_address)?;
		let stream_clone = stream.try_clone().unwrap();
		let (sender, receiver) = std::sync::mpsc::channel();
		std::thread::Builder::new()
			.name("Tcp Client Listen Thread".to_string())
			.spawn(move || Self::listen_thread(stream_clone, sender))
			.unwrap();

		Ok(Self {
			stream,
			transport_msg_receiver: receiver,
			server_disconnected: false,
		})
	}

	fn listen_thread(mut stream: TcpStream, sender: Sender<ClientTransportEvent>) {
		let mut buffer = [0; MAX_MSG_SIZE];

		loop {
			match stream.read(&mut buffer) {
				Ok(bytes_read) => {
					let client_disconnected = bytes_read == 0;

					if client_disconnected {
						let _ = sender.send(ClientTransportEvent::ServerDisconnected);
						return;
					}
					else {
						if sender.send(
							ClientTransportEvent::NewMsg(buffer[..bytes_read].to_vec())
						).is_err() {
							return;
						}
					};
				},
				Err(e) => {
					match e.kind() {
						io::ErrorKind::ConnectionAborted | io::ErrorKind::ConnectionRefused | io::ErrorKind::ConnectionReset => {
							let _ = sender.send(ClientTransportEvent::ServerDisconnected);
							return;
						}
						_ => {
							if sender.send(ClientTransportEvent::FailedToReceiveMsg(e)).is_err() {
								return;
							}
						},
					}
				}
			}
		}
	}
}

impl ClientTransport for TcpClientTransport {
	fn receive_event(&mut self) -> Option<ClientTransportEvent> {
		if self.server_disconnected {
			Some(ClientTransportEvent::ServerDisconnected)
		}
		else {
			self.transport_msg_receiver.try_recv().ok()
		}
	}

	fn send(&mut self, data: &[u8]) {
		assert!(data.len() < MAX_MSG_SIZE, "Sending packets over {MAX_MSG_SIZE} bytes is not supported: see MAX_MSG_SIZE!");

		if self.stream.write_all(data).is_err() {
			self.server_disconnected = true;
		}
	}
}

pub struct TcpServerTransport {
	transport_msg_receiver: Receiver<ServerTransportEvent>,
	client_streams: Arc<Mutex<HashMap<SocketAddr, TcpStream>>>,
}

impl TcpServerTransport {
	pub fn bind_port(port: u16) -> io::Result<Self> {
		Self::new(("0.0.0.0", port))
	}

	pub fn new<A: ToSocketAddrs>(address: A) -> io::Result<Self> {
		let listener = TcpListener::bind(address)?;
		let (send_channel, receive_channel) = std::sync::mpsc::channel();
		let client_streams = Arc::new(Mutex::new(HashMap::new()));
		let client_streams_clone = client_streams.clone();
		std::thread::Builder::new()
			.name("Tcp Listen Thread".to_string())
			.spawn(move || Self::listen_thread(send_channel, listener, client_streams_clone))
			.unwrap();

		Ok(Self {
			transport_msg_receiver: receive_channel,
			client_streams,
		})
	}

	fn handle_client_thread(mut stream: TcpStream, address: SocketAddr, sender: Arc<Sender<ServerTransportEvent>>) {
		let mut buffer = [0; MAX_MSG_SIZE];
		if sender.send(ServerTransportEvent::NewClient(address)).is_err() {
			return;
		}

		loop {
			match stream.read(&mut buffer) {
				Ok(bytes_read) => {
					let client_disconnected = bytes_read == 0;

					if client_disconnected {
						let _ = sender.send(ServerTransportEvent::ClientDisconnected(address));
						return;
					}
					else {
						if sender.send(
							ServerTransportEvent::NewMsg(
								TransportMsg {
									sender_address: address,
									data: buffer[..bytes_read].to_vec()
								}
							)
						).is_err() {
							return;
						}
					};
				},
				Err(e) => {
					match e.kind() {
						io::ErrorKind::ConnectionAborted | io::ErrorKind::ConnectionRefused | io::ErrorKind::ConnectionReset => {
							let _ = sender.send(ServerTransportEvent::ClientDisconnected(address));
							return;
						}
						_ => {
							if sender.send(ServerTransportEvent::FailedToReceiveMsg(e)).is_err() {
								return;
							}
						},
					}
				}
			}
		}
	}

	fn listen_thread(sender: Sender<ServerTransportEvent>, listener: TcpListener, client_streams: Arc<Mutex<HashMap<SocketAddr, TcpStream>>>) {
		let sender = Arc::new(sender);

		for stream in listener.incoming() {
			match stream {
				Ok(stream) => {
					let sender_clone = sender.clone();
					let address = stream.peer_addr().unwrap();
					let stream_clone = stream.try_clone().unwrap();
					client_streams.lock().unwrap().insert(address, stream);
					std::thread::Builder::new()
						.name("Client Tcp Thread".to_string())
						.spawn(move || Self::handle_client_thread(stream_clone, address, sender_clone))
						.unwrap();
				},
				Err(e) => {
					if sender.send(ServerTransportEvent::FailedToAcceptConnection(e)).is_err() {
						return;
					}
				},
			}
		}
	}
}

impl ServerTransport for TcpServerTransport {
	fn receive_event(&mut self) -> Option<ServerTransportEvent> {
		self.transport_msg_receiver.try_recv().ok()
	}

	fn send(&mut self, address: std::net::SocketAddr, data: &[u8]) {
		if let Ok(client_streams) = &mut self.client_streams.lock() {
			if let Some(stream) = client_streams.get_mut(&address) {
				let _ =stream.write_all(data);
			}
		}
	}
}
