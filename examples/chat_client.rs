use client_server::client::Client;
use std::{collections::VecDeque, sync::{Arc, Mutex}};

fn input_thread(messages_to_send: Arc<Mutex<VecDeque<String>>>) {
    loop {
        let mut input_str = String::new();
        std::io::stdin().read_line(&mut input_str).expect("Failed to read line");
        let input = input_str.trim();
        messages_to_send.lock().unwrap().push_front(input.to_string());
   }
}

fn main() {
    let messages_to_send: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
    let messages_to_send_clone = messages_to_send.clone();
    std::thread::spawn(move || input_thread(messages_to_send_clone));

    let mut client = Client::connect_localhost(8080).expect("Failed to connect to 127.0.0.1:8080");
    
    client.send(b"Connected").expect("Failed to send 'Connected' to server");
	
    loop {
        {
            let mut messages_to_send = messages_to_send.lock().unwrap();
            
            let mut exit = false;
            while let Some(input) = messages_to_send.pop_back() {
                if input == "/exit" {
                    exit = true;
                    break;
                }
                client.send(input.as_bytes()).expect("Failed to write to server");
            }

            if exit {
                break;
            }
        }

        while let Some(packet) = client.get_packet() {
            println!("{}", packet.data);
        }
    }
}
