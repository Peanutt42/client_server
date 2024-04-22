use client_server::{Client, ClientEvent, transport::ClientTransport};
use crate::{ClientToServerMsg, spawn_press_enter_to_quit_thread};

pub fn run(client_transport: Box<dyn ClientTransport>) {
	let mut client = Client::new(client_transport);

	println!("Successfully connected to server!");

	let exit_input_receiver = spawn_press_enter_to_quit_thread();

	loop {
		if exit_input_receiver.try_recv().is_ok() {
			break;
		}

		while let Some(event) = client.handle_event() {
			match event {
				ClientEvent::ServerDisconnected => {
					println!("Server disconnected");
					return;
				},
			}
		}

		println!("Sending \"Hello, World!\" to the server");
		let hello_world_msg = ClientToServerMsg::TextMsg("Hello, World!".to_string());
		client.send(hello_world_msg).unwrap();

		std::thread::sleep(std::time::Duration::from_secs(1));
	}
}
