use std::collections::HashSet;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::io;
use std::sync::mpsc::{Receiver, Sender};
use serde::{Deserialize, Serialize};

use crate::transport::{MAX_MSG_SIZE, ClientTransport, ServerTransport, TransportMsg, ServerTransportEvent};

#[derive(Serialize, Deserialize)]
enum UdpMsg {
	InnerMsg(Vec<u8>),
	Disconnected
}

// TODO: once we combine / refactor udp transport and have the client receive from the server, add a udp_msg::Disconnected like with the client already
pub struct UdpClientTransport {
	socket: UdpSocket,
}

impl UdpClientTransport {
	pub fn new<A: ToSocketAddrs>(address: A) -> io::Result<Self> {
		let socket = UdpSocket::bind("0.0.0.0:0")?;
		socket.connect(address)?;

		Ok(Self {
			socket,
		})
	}
}

impl Drop for UdpClientTransport {
	fn drop(&mut self) {
		let _ = self.socket.send(&bincode::serialize(&UdpMsg::Disconnected).unwrap());
	}
}

impl ClientTransport for UdpClientTransport {
	fn send(&mut self, data: &[u8]) -> io::Result<()> {
		let udp_msg = UdpMsg::InnerMsg(data.to_vec());
		match bincode::serialize(&udp_msg) {
			Ok(data) => {
				assert!(data.len() < MAX_MSG_SIZE, "Sending packets over {MAX_MSG_SIZE} bytes is not supported: see MAX_MSG_SIZE!");
				self.socket.send(&data).map(|_| ())
			},
			Err(e) => Err(io::Error::other(e)),
		}
	}
}

pub struct UdpServerTransport {
	transport_msg_receiver: Receiver<ServerTransportEvent>,
}

impl UdpServerTransport {
	pub fn bind_port(port: u16) -> io::Result<Self> {
		Self::new(("0.0.0.0", port))
	}

	pub fn new<A: ToSocketAddrs>(address: A) -> io::Result<Self> {
		let socket = UdpSocket::bind(address)?;
		let (send_channel, receive_channel) = std::sync::mpsc::channel();
		std::thread::Builder::new()
			.name("Udp Listen Thread".to_string())
			.spawn(move || Self::listen_thread(socket, send_channel))
			.unwrap();

		Ok(Self {
			transport_msg_receiver: receive_channel,
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
						let res = sender.send(ServerTransportEvent::NewClient(address.ip()));
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
											sender_address: address.ip(),
											data,
										}
									)
								},
								UdpMsg::Disconnected => ServerTransportEvent::ClientDisconnected(address.ip()),
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
}
