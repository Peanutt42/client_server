use crate::internal::{InternalClientPacket, InternalServerPacket};
use crate::protocol::{NetworkingListener, NetworkingStream};
use crate::server::{Error, Result, Server};
use crate::{ClientId, ClientMessage, ClientPacket, MAX_MESSAGE_SIZE};
use std::collections::{HashMap, VecDeque};
use std::io;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct ClientData {
    pub stream: NetworkingStream,
    pub ping_send_time: Option<Instant>,
    pub last_ping: Option<Duration>,
}

impl ClientData {
    pub fn new(stream: NetworkingStream) -> Self {
        Self {
            stream,
            ping_send_time: None,
            last_ping: None,
        }
    }
}

pub struct SharedData {
    pub clients: HashMap<ClientId, ClientData>,
    pub next_id: ClientId,

    pub ping_interval: Option<Duration>,

    pub packets: VecDeque<ClientPacket>,

    pub error_log: Vec<Error>,
}

impl SharedData {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            next_id: 0,
            ping_interval: None,
            packets: VecDeque::new(),
            error_log: Vec::new(),
        }
    }

    pub fn add_client(&mut self, stream: NetworkingStream) -> ClientId {
        let client_id = self.next_id;
        self.next_id += 1;

        self.clients.insert(client_id, ClientData::new(stream));

        client_id
    }

    pub fn remove_client(&mut self, client_id: ClientId) {
        self.clients.remove(&client_id);
    }

    pub fn client_disconnected(&mut self, client_id: ClientId) {
        self.packets
            .push_back(ClientPacket::new(client_id, ClientMessage::Disconnected));
        self.broadcast(
            &InternalServerPacket::ClientDisconnected(client_id),
            client_id,
        )
        .expect("failed to broadcast that a client has disconnected to all the other clients");
        self.remove_client(client_id);
    }

    pub fn stream_send(
        packet: &InternalServerPacket,
        stream: &mut NetworkingStream,
        client_id: ClientId,
    ) -> Result<()> {
        match bincode::serialize(packet) {
            Ok(buffer) => {
                if let Err(e) = stream.write(buffer.as_slice()) {
                    Err(Error::SendError {
                        client_id,
                        io_error: e,
                    })
                } else {
                    Ok(())
                }
            }
            Err(e) => Err(Error::SerializePacket {
                bincode_error: e.to_string(),
            }),
        }
    }

    pub fn send(&mut self, packet: &InternalServerPacket, client_id: ClientId) -> Result<()> {
        match bincode::serialize(packet) {
            Ok(buffer) => match self.clients.get_mut(&client_id) {
                Some(client) => {
                    if let Err(e) = client.stream.write(buffer.as_slice()) {
                        Err(Error::SendError {
                            client_id,
                            io_error: e,
                        })
                    } else {
                        Ok(())
                    }
                }
                None => Err(Error::InvalidClientId { client_id }),
            },
            Err(e) => Err(Error::SerializePacket {
                bincode_error: e.to_string(),
            }),
        }
    }

    pub fn broadcast_all(&mut self, packet: &InternalServerPacket) -> Result<()> {
        match bincode::serialize(packet) {
            Ok(buffer) => {
                let mut error_msg = Ok(());
                for (client_id, client) in self.clients.iter_mut() {
                    if let Err(e) = client.stream.write(buffer.as_slice()) {
                        error_msg = Err(Error::SendError {
                            client_id: *client_id,
                            io_error: e,
                        });
                    }
                }
                error_msg
            }
            Err(e) => Err(Error::SerializePacket {
                bincode_error: e.to_string(),
            }),
        }
    }

    /// if you want to broadcast to all clients, use `broadcast_all`
    pub fn broadcast(
        &mut self,
        packet: &InternalServerPacket,
        ignored_client: ClientId,
    ) -> Result<()> {
        match bincode::serialize(packet) {
            Ok(buffer) => {
                let mut error_msg = Ok(());
                for (client_id, client) in self.clients.iter_mut() {
                    if *client_id != ignored_client {
                        if let Err(e) = client.stream.write(buffer.as_slice()) {
                            error_msg = Err(Error::SendError {
                                client_id: *client_id,
                                io_error: e,
                            });
                        }
                    }
                }
                error_msg
            }
            Err(e) => Err(Error::SerializePacket {
                bincode_error: e.to_string(),
            }),
        }
    }
}

impl Default for SharedData {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    pub fn listen_thread(mut listener: NetworkingListener, shared_data: Arc<Mutex<SharedData>>) {
        listener
            .set_nonblocking(true)
            .expect("failed to set nonblocking");
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let shared_data_clone = shared_data.clone();
                    std::thread::spawn(move || {
                        Self::handle_client_thread(stream, shared_data_clone)
                    });
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => (),
                Err(e) => {
                    if let Some(code) = e.raw_os_error() {
                        // (only caused on windows on shutdown)
                        // error message: "Either the application has not called WSAStartup, or WSAStartup failed. (os error 10093)"
                        // i could not find a fix for this, it doesn't seem to matter that much
                        if code == 10093 {
                            break;
                        }
                    }
                    shared_data
                        .lock()
                        .unwrap()
                        .error_log
                        .push(Error::ErrorAcceptingConnection { io_error: e });
                }
            }
        }
    }

    pub fn handle_client_thread(mut stream: NetworkingStream, shared_data: Arc<Mutex<SharedData>>) {
        let mut buffer = [0; MAX_MESSAGE_SIZE];
        let client_id: ClientId;

        {
            let mut shared_data = shared_data.lock().unwrap();
            client_id = shared_data.add_client(stream.try_clone().expect("Failed to clone stream"));
            shared_data
                .packets
                .push_front(ClientPacket::new(client_id, ClientMessage::Connected));
            shared_data
                .send(&InternalServerPacket::ConnectResponse(client_id), client_id)
                .expect("failed to send the connect response to client");
            shared_data
                .broadcast(
                    &InternalServerPacket::NewClientConnected(client_id),
                    client_id,
                )
                .expect("failed to send 'ClientConnected' to all other clients");
        }

        loop {
            match stream.read(&mut buffer) {
                Ok(bytes_read) => {
                    if bytes_read == 0 {
                        break;
                    }

                    match bincode::deserialize(&buffer[..bytes_read]) {
                        Ok(packet) => {
                            let mut shared_data = shared_data.lock().unwrap();
                            match packet {
                                InternalClientPacket::BroadcastMessage(message) => {
                                    shared_data.packets.push_back(ClientPacket::new(
                                        client_id,
                                        ClientMessage::ClientToClientMessage(message.clone()),
                                    ));
                                    if let Err(e) = shared_data.broadcast(
                                        &InternalServerPacket::ClientToClient(client_id, message),
                                        client_id,
                                    ) {
                                        shared_data.error_log.push(e);
                                    }
                                }
                                InternalClientPacket::PersonalMessage(
                                    target_client_id,
                                    message,
                                ) => {
                                    shared_data.packets.push_back(ClientPacket::new(
                                        client_id,
                                        ClientMessage::ClientToClientMessage(message.clone()),
                                    ));
                                    if let Err(e) = shared_data.send(
                                        &InternalServerPacket::ClientToClient(client_id, message),
                                        target_client_id,
                                    ) {
                                        shared_data.error_log.push(e);
                                    }
                                }
                                InternalClientPacket::ServerMessage(message) => {
                                    shared_data.packets.push_back(ClientPacket::new(
                                        client_id,
                                        ClientMessage::ClientToServerMessage(message),
                                    ));
                                }
                                InternalClientPacket::PingRespone => {
                                    if let Some(client) = shared_data.clients.get_mut(&client_id) {
                                        if let Some(ping_send_time) = client.ping_send_time {
                                            let round_trip_time = Instant::now() - ping_send_time;
                                            client.last_ping = Some(round_trip_time.div_f32(2.0));
                                        }
                                    }
                                }
                            };
                        }
                        Err(e) => {
                            shared_data
                                .lock()
                                .unwrap()
                                .error_log
                                .push(Error::DeserializePacket {
                                    bincode_error: e.to_string(),
                                })
                        }
                    }
                }
                Err(ref e)
                    if e.kind() == io::ErrorKind::WouldBlock
                        || e.kind() == io::ErrorKind::ConnectionReset
                        || e.kind() == io::ErrorKind::ConnectionAborted =>
                {
                    ()
                }
                Err(e) => {
                    if let Some(code) = e.raw_os_error() {
                        // (only caused on windows on shutdown)
                        // error message: "Either the application has not called WSAStartup, or WSAStartup failed. (os error 10093)"
                        // i could not find a fix for this, it doesn't seem to matter that much
                        if code == 10093 {
                            break;
                        }
                    }
                    shared_data
                        .lock()
                        .unwrap()
                        .error_log
                        .push(Error::ReadError {
                            client_id,
                            io_error: e,
                        });
                    break;
                }
            }
        }
        shared_data.lock().unwrap().client_disconnected(client_id);
    }
}
