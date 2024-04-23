use serde::{Deserialize, Serialize};

pub mod client;
pub mod server;

pub const PORT: u16 = 5000;

#[derive(Serialize, Deserialize)]
pub enum ClientToServerMsg {
	TextMsg(String),
}

#[derive(Serialize, Deserialize)]
pub enum ServerToClientMsg {
	TextMsgReceived,
}

pub fn spawn_press_enter_to_quit_thread() -> std::sync::mpsc::Receiver<()> {
	let (sender, reciever) = std::sync::mpsc::channel();

	std::thread::Builder::new()
    	.name("Input Listen Thread".to_string())
     	.spawn(move || {
    		let mut line = String::new();
			std::io::stdin().read_line(&mut line).unwrap();
			let _ = sender.send(());
      	})
        .unwrap();

	reciever
}
