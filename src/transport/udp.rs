use std::collections::HashSet;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::io;
use std::sync::mpsc::{Receiver, Sender};
use serde::{Deserialize, Serialize};

use crate::transport::{MAX_MSG_SIZE, ClientTransport, ClientTransportEvent, ServerTransport, ServerTransportEvent, TransportMsg};

#[derive(Serialize, Deserialize)]
enum UdpMsg {
	InnerMsg(Vec<u8>),
	Disconnected
}

pub struct UdpClientTransport {
	socket: UdpSocket,
	transport_msg_receiver: Receiver<ClientTransportEvent>,
}

impl UdpClientTransport {
	pub fn new<A: 'static + Clone + Sync + Send + ToSocketAddrs>(address: A) -> io::Result<Self> {
		let socket = UdpSocket::bind("0.0.0.0:0")?;
		socket.connect(address.clone())?;
		let socket_clone = socket.try_clone().unwrap();
		let (sender, receiver) = std::sync::mpsc::channel();
		std::thread::Builder::new()
			.name("Udp Client Listen Thread".to_string())
			.spawn(move || Self::listen_thread(socket_clone, address, sender))
			.unwrap();

		Ok(Self {
			socket,
			transport_msg_receiver: receiver,
		})
	}

	fn listen_thread<A: 'static + Sync + Send + ToSocketAddrs>(socket: UdpSocket, server_address: A, sender: Sender<ClientTransportEvent>) {
		let mut buffer = [0; MAX_MSG_SIZE];
		let mut server_addresses = HashSet::new();
		for address in server_address.to_socket_addrs().unwrap() {
			server_addresses.insert(address);
		}

		loop {
			match socket.recv_from(&mut buffer) {
				Ok((bytes_read, address)) => {
					if !server_addresses.contains(&address) {
						continue;
					}

					match bincode::deserialize(&buffer[..bytes_read]) {
						Ok(udp_msg) => {
							let event = match udp_msg {
								UdpMsg::InnerMsg(data) => ClientTransportEvent::NewMsg(data),
								UdpMsg::Disconnected => ClientTransportEvent::ServerDisconnected,
							};
							if sender.send(event).is_err() {
								break;
							}
						},
						Err(e) => {
							let res = sender.send(ClientTransportEvent::FailedToReceiveMsg(io::Error::other(e)));
							if res.is_err() {
								break;
							}
						}
					}
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

		let _ = socket.send(&bincode::serialize(&UdpMsg::Disconnected).unwrap());
	}
}

impl Drop for UdpClientTransport {
	fn drop(&mut self) {
		let _ = self.socket.send(&bincode::serialize(&UdpMsg::Disconnected).unwrap());
	}
}

impl ClientTransport for UdpClientTransport {
	fn receive_event(&mut self) -> Option<ClientTransportEvent> {
		self.transport_msg_receiver.try_recv().ok()
	}

	fn send(&mut self, data: &[u8]) {
		if let Ok(data) = bincode::serialize(&UdpMsg::InnerMsg(data.to_vec())) {
			assert!(data.len() < MAX_MSG_SIZE, "Sending packets over {MAX_MSG_SIZE} bytes is not supported: see MAX_MSG_SIZE!");
			self.socket.send(&data).unwrap();
		}
	}
}

pub struct UdpServerTransport {
	transport_msg_receiver: Receiver<ServerTransportEvent>,
	socket: UdpSocket,
}

impl UdpServerTransport {
	pub fn bind_port(port: u16) -> io::Result<Self> {
		Self::new(("0.0.0.0", port))
	}

	pub fn new<A: ToSocketAddrs>(address: A) -> io::Result<Self> {
		let socket = UdpSocket::bind(address)?;
		let socket_clone = socket.try_clone().unwrap();
		let (send_channel, receive_channel) = std::sync::mpsc::channel();
		std::thread::Builder::new()
			.name("Udp Listen Thread".to_string())
			.spawn(move || Self::listen_thread(socket_clone, send_channel))
			.unwrap();

		Ok(Self {
			transport_msg_receiver: receive_channel,
			socket,
		})
	}

	fn listen_thread(socket: UdpSocket, sender: Sender<ServerTransportEvent>) {
		let mut buffer = [0; MAX_MSG_SIZE];
		let mut connected_clients: HashSet<SocketAddr> = HashSet::new();

		loop {
			match socket.recv_from(&mut buffer) {
				Ok((bytes_read, address)) => {
					if !connected_clients.contains(&address) {
						connected_clients.insert(address);
						let res = sender.send(ServerTransportEvent::NewClient(address));
						if res.is_err() {
							break;
						}
					}

					match bincode::deserialize(&buffer[..bytes_read]) {
						Ok(udp_msg) => {
							let event = match udp_msg {
								UdpMsg::InnerMsg(data) => {
									ServerTransportEvent::NewMsg(
										TransportMsg {
											sender_address: address,
											data,
										}
									)
								},
								UdpMsg::Disconnected => ServerTransportEvent::ClientDisconnected(address),
							};
							let res = sender.send(event);
							if res.is_err() {
								break;
							}
						},
						Err(e) => {
							let res = sender.send(ServerTransportEvent::FailedToReceiveMsg(io::Error::other(e)));
							if res.is_err() {
								break;
							}
						}
					}
				},
				Err(e) => {
					if sender.send(ServerTransportEvent::FailedToReceiveMsg(e)).is_err() {
						return;
					}
				}
			}
		}

		let _ = socket.send(&bincode::serialize(&UdpMsg::Disconnected).unwrap());
	}
}


impl ServerTransport for UdpServerTransport {
	fn receive_event(&mut self) -> Option<ServerTransportEvent> {
		self.transport_msg_receiver.try_recv().ok()
	}

	fn send(&mut self, address: SocketAddr, data: &[u8]) {
		if let Ok(data) = bincode::serialize(&UdpMsg::InnerMsg(data.to_vec())) {
			assert!(data.len() < MAX_MSG_SIZE, "Sending packets over {MAX_MSG_SIZE} bytes is not supported: see MAX_MSG_SIZE!");
			self.socket.send_to(&data, address).unwrap();
		}
	}
}
