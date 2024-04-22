use client_server::transport::tcp::TcpClientTransport;
use example_chat::PORT;

fn main() {
	println!("Running as client!");
	let client_transport = TcpClientTransport::new(("127.0.0.1", PORT))
		.expect("Failed to connect to the server!");
	example_chat::client::run(Box::new(client_transport));
}
