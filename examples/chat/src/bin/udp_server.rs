use client_server::transport::udp::UdpServerTransport;
use example_chat::PORT;

fn main() {
	println!("Running as server!");
	let server_transport = UdpServerTransport::bind_port(PORT).unwrap();
	example_chat::server::run(Box::new(server_transport));
}
