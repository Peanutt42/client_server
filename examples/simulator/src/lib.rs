use serde::{Deserialize, Serialize};



pub const PORT: u16 = 6000;

#[derive(Serialize, Deserialize)]
pub struct ClientToServerMsg {
	msg: String,
}

impl ClientToServerMsg {
	pub fn new() -> Self {
		Self {
			msg: "Hello, World!".to_string(),
		}
	}
}
