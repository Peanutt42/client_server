use client_server::{Server, ClientMessage};
use std::{collections::VecDeque, sync::{Arc, Mutex}};

fn input_thread(input_commands: Arc<Mutex<VecDeque<String>>>) {
	loop {
		let mut input_str = String::new();
		std::io::stdin().read_line(&mut input_str).expect("Failed to read line");
		let input = input_str.trim();
		input_commands.lock().unwrap().push_front(input.to_string());
   }
}

fn main() {
	let mut server = Server::bind("0.0.0.0:8080").expect("failed to bind server to 0.0.0.0:8080");

	let input_commands: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
	let input_commands_clone = input_commands.clone();
	std::thread::spawn(move || input_thread(input_commands_clone));

	loop {
		{
			let mut input_commands = input_commands.lock().unwrap();

			let mut exit = false;
			while let Some(input) = input_commands.pop_back() {
				if input == "/exit" {
					exit = true;
					break;
				}
				else if input.starts_with("/kick ") {
                    if let Some(input) = input.split_whitespace().nth(1) {
                        if let Ok(client_id) = input.parse() {
							server.kick_client(client_id);
						}
						else {
							eprintln!("client_id argument was not a number");
						}
                    }
                    else {
                        eprintln!("invalid command usage: /kick [client_id]");
                    }
				}
				else {
					server.broadcast_all(&Vec::from(input.as_bytes())).expect("failed to write to all clients");
				}			
			}

			if exit {
				break;
			}
		}

		while let Some(packet) = server.get_packet() {
			let client_address = server.get_client_ip_address(packet.author).unwrap_or("unkown".to_string());
			match packet.message {
				ClientMessage::Connected => {
					println!("New client {}: {client_address}", packet.author);
				}
				ClientMessage::ClientToClientMessage(message) => {
					println!("{client_address} [{}]: {}", packet.author, String::from_utf8(message).unwrap());
				},
				ClientMessage::ClientToServerMessage(message) => {
					println!("{client_address} [{}] (only to server): {}", packet.author, String::from_utf8(message).unwrap());
				}
				ClientMessage::Disconnected => {
					println!("Client {} disconnected", packet.author);
				}
			}
			
		}
		
		for e in server.get_error_log() {
			eprintln!("error: {e}");
		}
	}
}