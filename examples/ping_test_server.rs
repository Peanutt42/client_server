use std::time::Duration;

use client_server::{Server, ClientMessage};

fn main() {
	let mut server = Server::bind("0.0.0.0:8080").expect("failed to bind server to 0.0.0.0:8080");

	server.set_ping_interval(Duration::from_secs(1));

	loop {
		while let Some(packet) = server.get_packet() {
			let client_address = server.get_client_ip_address(packet.author).unwrap_or("unkown".to_string());
			match packet.message {
				ClientMessage::Connected => {
					println!("New client {}: {client_address}", packet.author);
				}
				ClientMessage::Disconnected => {
					println!("Client {} disconnected", packet.author);
				},
				_ => {},
			}
		}

		if let Err(e) = server.update() {
			eprintln!("failed to update: {e}");
		}
		else {
			for client_id in server.get_clients() {
				if let Some(ping) = server.get_ping(client_id) {
					println!("{client_id}: {}ms", ping.as_secs_f32() * 1000.0);
				}
			}
		}
		if let Some(interval) = server.get_ping_interval() {
			std::thread::sleep(interval);
		}
	}
}