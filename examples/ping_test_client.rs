use client_server::{Client, ServerPacket};

fn main() {
	let mut client = Client::connect_tcp("127.0.0.1:8080").expect("Failed to connect to 127.0.0.1:8080");

	loop {
		let mut exit = false;
		while let Some(packet) = client.get_packet() {
			match packet {
				ServerPacket::ConnectedSuccessfully => println!("Successfully connected to server"),
				ServerPacket::YouWereKicked => {
					println!("You were kicked!");
					exit = true;
					break;
				},
				ServerPacket::Disconnected => {
					println!("Server disconnected!");
					exit = true;
					break;
				},
				ServerPacket::ConnectionRefused => {
					eprintln!("Server refused connection!");
					exit = true;
					break;
				},
				_ => {},
			}
		}

		for e in client.get_error_log() {
			eprintln!("error: {e}");
		}

		if exit {
            break;
        }
	}
}