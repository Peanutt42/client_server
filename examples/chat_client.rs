use client_server::{Client, ServerPacket};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

fn input_thread(messages_to_send: Arc<Mutex<VecDeque<String>>>) {
    loop {
        let mut input_str = String::new();
        std::io::stdin()
            .read_line(&mut input_str)
            .expect("Failed to read line");
        let input = input_str.trim();
        messages_to_send
            .lock()
            .unwrap()
            .push_front(input.to_string());
    }
}

fn main() {
    let messages_to_send: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
    let messages_to_send_clone = messages_to_send.clone();
    std::thread::spawn(move || input_thread(messages_to_send_clone));

    let mut client =
        Client::connect_tcp("127.0.0.1:8080").expect("Failed to connect to 127.0.0.1:8080");

    loop {
        {
            let mut messages_to_send = messages_to_send.lock().unwrap();

            let mut exit = false;
            while let Some(input) = messages_to_send.pop_back() {
                if input == "/exit" {
                    exit = true;
                    break;
                }
                if input.starts_with("/server ") {
                    client
                        .send_to_server(&Vec::from(input.trim_start_matches("/server ")))
                        .expect("Failed to write to server");
                } else {
                    client
                        .send_to_all(&Vec::from(input.as_bytes()))
                        .expect("Failed to write to all clients");
                }
            }

            if exit {
                break;
            }
        }

        let mut exit = false;
        while let Some(packet) = client.get_packet() {
            match packet {
                ServerPacket::ConnectedSuccessfully => println!("Successfully connected to server"),
                ServerPacket::NewClientConnected(new_client_id) => {
                    println!("New client connected: {new_client_id}")
                }
                ServerPacket::ClientDisconnected(client_id) => {
                    println!("Client {client_id} disconnected")
                }
                ServerPacket::ClientKicked(client_id) => println!("Client {client_id} was kicked"),
                ServerPacket::YouWereKicked => {
                    println!("You were kicked!");
                    exit = true;
                    break;
                }
                ServerPacket::ClientToClientMessage(client_id, message) => {
                    println!("{client_id}: {}", String::from_utf8(message).unwrap())
                }
                ServerPacket::ServerToClientMessage(message) => {
                    println!("Server: {}", String::from_utf8(message).unwrap())
                }
                ServerPacket::Disconnected => {
                    println!("Server disconnected!");
                    exit = true;
                    break;
                }
                ServerPacket::ConnectionRefused => {
                    println!("Server refused connection!");
                    exit = true;
                    break;
                }
            }
        }

        for e in client.get_error_log() {
            println!("error: {e}");
        }

        if exit {
            break;
        }
    }
}
