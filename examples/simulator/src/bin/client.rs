use std::time::Duration;
use client_server::{Client, transport::{udp::UdpClientTransport, simulator::ClientSimulatorTransport}};
use simulator::{PORT, ClientToServerMsg};

fn main() {
	println!("Running as client!");
	let client_transport = ClientSimulatorTransport::new(0.1, Duration::from_secs_f32(1.0), UdpClientTransport::new(("127.0.0.1", PORT))
		.expect("Failed to connect to the server!"));
	let mut client = Client::new(Box::new(client_transport));

	loop {
		println!("Sending \"Hello, World!\" to the server  BUT with 10% packet loss and 1000ms latency");
		client.send(&ClientToServerMsg::new());
		let mut _line = String::new();
		std::io::stdin().read_line(&mut _line).unwrap();
	}
}
