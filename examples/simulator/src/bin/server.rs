use std::{collections::HashMap, time::Instant};
use client_server::{Server, ServerEvent, transport::udp::UdpServerTransport};
use simulator::{PORT, ClientToServerMsg};

fn main() {
	println!("Running as server!");
	let server_transport = UdpServerTransport::bind_port(PORT).unwrap();
	let mut server = Server::new(Box::new(server_transport));

	let mut client_last_packet_time = HashMap::new();

	loop {
		while let Some(event) = server.receive_event::<ClientToServerMsg>() {
			match event {
				ServerEvent::NewClient(client_id) => println!("New client connected: {client_id}"),
				ServerEvent::ClientDisconnected(client_id) => println!("Client disconnected: {client_id}"),
				ServerEvent::FailedToParseMsg(client_id) => eprintln!("Failed to parse msg from {client_id}"),
				ServerEvent::FailedToAcceptConnection(e) => eprintln!("Failed to accept connection: {e}"),
				ServerEvent::FailedToReceiveMsg(e) => eprintln!("Failed to receive msg: {e}"),

				ServerEvent::NewMsg(client_msg) => {
					let last_packet_time = client_last_packet_time.entry(client_msg.client_id).or_insert(Instant::now());
					let duration = (Instant::now() - *last_packet_time).as_secs_f32();
					println!("{}: {} packets/s", client_msg.client_id, 1.0 / duration);
					*last_packet_time = Instant::now();
				},
			}
		}
	}
}
