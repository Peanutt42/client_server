use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};

use crate::transport::{ClientTransport, ClientTransportEvent};

pub struct ClientSimulatorTransport<T: ClientTransport> {
	transport: Arc<Mutex<T>>,
	packet_sender: Sender<(Instant, Vec<u8>)>,
	packet_loss_percentage: f32,
	latency: Duration,
}

impl<T: 'static + Send + ClientTransport> ClientSimulatorTransport<T> {
	pub fn new(packet_loss_percentage: f32, latency: Duration, transport: T) -> Self {
		let transport = Arc::new(Mutex::new(transport));
		let transport_clone = transport.clone();
		let (sender, receiver) = std::sync::mpsc::channel();
		std::thread::Builder::new()
			.name("Networking Simulator Thread".to_string())
			.spawn(move || Self::delayed_packet_send_thread(receiver, transport_clone))
			.unwrap();

		Self {
			transport,
			packet_sender: sender,
			packet_loss_percentage,
			latency,
		}
	}

	fn delayed_packet_send_thread(packet_receiver: Receiver<(Instant, Vec<u8>)>, transport: Arc<Mutex<T>>) {
		let mut pending_packets = VecDeque::new();

		loop {
			match packet_receiver.try_recv() {
				Ok((send_time, data)) => {
					pending_packets.push_back((send_time, data));
				},
				Err(e) => {
					match e {
						TryRecvError::Empty => {},
						TryRecvError::Disconnected => return,
					}
				}
			}

			let send_packet =
			if let Some((send_time, _data)) = pending_packets.front() {
				*send_time <= Instant::now()
			}
			else {
				false
			};
			if send_packet {
				let (_send_time, data) = pending_packets.pop_front().unwrap();
				transport.lock().unwrap().send(&data);
			}
		}
	}

	fn should_send_packet(&self) -> bool {
		let zero_to_one_chance = rand::random::<f32>();
		zero_to_one_chance > self.packet_loss_percentage
	}
}

impl<T: 'static + Send + ClientTransport> ClientTransport for ClientSimulatorTransport<T> {
	fn send(&mut self, data: &[u8]) {
		if self.should_send_packet() {
			self.packet_sender.send((Instant::now() + self.latency, data.to_vec())).unwrap();
		}
	}

	fn receive_event(&mut self) -> Option<ClientTransportEvent> {
		self.transport.lock().unwrap().receive_event()
	}
}
