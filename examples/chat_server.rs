use client_server::server::Server;
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
    let mut server = Server::bind_localhost(8080).expect("failed to bind server to 127.0.0.1:8080");

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
            
                server.broadcast_all(format!("SERVER: {}", input).as_bytes()).expect("failed to write to all clients");
            }

            if exit {
                break;
            }
        }

        if let Some(packet) = server.get_packet() {
            let client_ip = server.get_client_ip_address(packet.sender).unwrap_or("unkown".to_string());
            println!("{}: {}", client_ip, packet.data);
            server.broadcast(format!("{client_ip}: {}", packet.data).as_bytes(), packet.sender).expect("failed to broadcast message to all clients");
        }
    }
}