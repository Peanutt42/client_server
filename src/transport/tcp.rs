use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::io::{self, Read, Write};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use crate::transport::{MAX_MSG_SIZE, ClientTransport, ServerTransport, TransportMsg, ServerTransportEvent};

pub struct TcpClientTransport {
	stream: TcpStream,
}

impl TcpClientTransport {
	pub fn new<A: ToSocketAddrs>(server_address: A) -> io::Result<Self> {
		let stream = TcpStream::connect(server_address)?;

		Ok(Self {
			stream,
		})
	}
}

impl ClientTransport for TcpClientTransport {
	fn send(&mut self, data: &[u8]) -> io::Result<()> {
		assert!(data.len() < MAX_MSG_SIZE, "Sending packets over {MAX_MSG_SIZE} bytes is not supported: see MAX_MSG_SIZE!");

		self.stream.write_all(data)
	}
}

pub struct TcpServerTransport {
	transport_msg_receiver: Receiver<ServerTransportEvent>,
}

impl TcpServerTransport {
	pub fn bind_port(port: u16) -> io::Result<Self> {
		Self::new(("0.0.0.0", port))
	}

	pub fn new<A: ToSocketAddrs>(address: A) -> io::Result<Self> {
		let listener = TcpListener::bind(address)?;
		let (send_channel, receive_channel) = std::sync::mpsc::channel();
		std::thread::Builder::new()
			.name("Tcp Listen Thread".to_string())
			.spawn(move || Self::listen_thread(send_channel, listener))
			.unwrap();

		Ok(Self {
			transport_msg_receiver: receive_channel,
		})
	}

	fn handle_client_thread(mut stream: TcpStream, sender: Arc<Sender<ServerTransportEvent>>) {
		let mut buffer = [0; MAX_MSG_SIZE];
		let address = stream.peer_addr().unwrap().ip();
		sender.send(ServerTransportEvent::NewClient(address)).unwrap();

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
						_ => sender.send(ServerTransportEvent::FailedToReceiveMsg(e)).unwrap(),
					}
				}
			}
		}
	}

	fn listen_thread(sender: Sender<ServerTransportEvent>, listener: TcpListener) {
		let sender = Arc::new(sender);

		for stream in listener.incoming() {
			match stream {
				Ok(stream) => {
					let sender_clone = sender.clone();
					std::thread::Builder::new()
						.name("Client Tcp Thread".to_string())
						.spawn(move || Self::handle_client_thread(stream, sender_clone))
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
}
