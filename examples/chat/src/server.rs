use client_server::{Server, ServerEvent, transport::ServerTransport};
use crate::{ClientToServerMsg, ServerToClientMsg, spawn_press_enter_to_quit_thread};

pub fn run(server_transport: Box<dyn ServerTransport>) {
	let mut server = Server::new(server_transport);

	let exit_input_receiver = spawn_press_enter_to_quit_thread();

	loop {
		if exit_input_receiver.try_recv().is_ok() {
			break;
		}

		while let Some(event) = server.receive_event::<ClientToServerMsg>() {
			match event {
				ServerEvent::NewClient(client_id) => println!("New client connected: {client_id}"),
				ServerEvent::ClientDisconnected(client_id) => println!("Client disconnected: {client_id}"),
				ServerEvent::FailedToParseMsg(client_id) => eprintln!("Failed to parse msg from {client_id}"),
				ServerEvent::FailedToAcceptConnection(e) => eprintln!("Failed to accept connection: {e}"),
				ServerEvent::FailedToReceiveMsg(e) => eprintln!("Failed to receive msg: {e}"),

				ServerEvent::NewMsg(client_msg) => {
					match client_msg.msg {
						ClientToServerMsg::TextMsg(text) => {
							println!("New msg from {}: {}", client_msg.sender_id, text);
							server.send_to(client_msg.sender_id, &ServerToClientMsg::TextMsgReceived)
						},
					}
				},
			}
		}
	}
}
