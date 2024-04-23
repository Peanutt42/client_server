use std::{io::Write, time::{Duration, Instant}};

use client_server::{Client, ClientEvent, transport::ClientTransport};
use crate::{ClientToServerMsg, ServerToClientMsg, spawn_press_enter_to_quit_thread};

pub fn run(client_transport: Box<dyn ClientTransport>) {
	let mut client = Client::new(client_transport);

	println!("Successfully connected to server!");
	println!("When the server reads our msg, it sends back a notification: '✅'");

	let exit_input_receiver = spawn_press_enter_to_quit_thread();

	let mut next_sent_msg_time = Instant::now();

	loop {
		if exit_input_receiver.try_recv().is_ok() {
			break;
		}

		while let Some(event) = client.handle_event() {
			match event {
				ClientEvent::ServerDisconnected => {
					println!("\nServer disconnected");
					return;
				},
				ClientEvent::FailedToParseMsg(e) => eprintln!("Faield to parse server msg: {e}"),
				ClientEvent::FailedToReceiveMsg(e) => eprintln!("Failed to receive server msg: {e}"),
				ClientEvent::MsgFromServer(msg) => {
					match msg {
						ServerToClientMsg::TextMsgReceived => println!(" ✅"),
					}
				}
			}
		}

		if next_sent_msg_time <= Instant::now() {
			print!("Sending \"Hello, World!\" to the server...");
			std::io::stdout().flush().unwrap();
			let hello_world_msg = ClientToServerMsg::TextMsg("Hello, World!".to_string());
			client.send(&hello_world_msg);
			next_sent_msg_time = Instant::now() + Duration::from_secs(1);
		}
	}
}
