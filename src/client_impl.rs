use crate::client::{Client, Error, Result};
use crate::internal::{InternalClientPacket, InternalServerPacket};
use crate::protocol::NetworkingStream;
use crate::{ClientId, ServerPacket, MAX_MESSAGE_SIZE};
use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Mutex};

pub struct SharedData {
    pub send_stream: NetworkingStream,
    pub packets: VecDeque<ServerPacket>,
    pub id: Option<ClientId>,
    pub error_log: Vec<Error>,
}

impl SharedData {
    pub fn new(send_stream: NetworkingStream) -> Self {
        Self {
            send_stream,
            packets: VecDeque::new(),
            id: None,
            error_log: Vec::new(),
        }
    }

    pub fn server_disconnected(&mut self) {
        self.packets.push_back(ServerPacket::Disconnected);
    }

    pub fn server_refused(&mut self) {
        self.packets.push_back(ServerPacket::ConnectionRefused);
    }

    pub fn send_packet(&mut self, packet: &InternalClientPacket) -> Result<()> {
        match bincode::serialize(&packet) {
            Ok(buffer) => {
                if let Err(e) = self.send_stream.write(buffer.as_slice()) {
                    Err(Error::SendError { io_error: e })
                } else {
                    Ok(())
                }
            }
            Err(e) => Err(Error::SerializePacket {
                bincode_error: e.to_string(),
            }),
        }
    }
}

impl Client {
    pub fn listen_thread(mut stream: NetworkingStream, shared_data: Arc<Mutex<SharedData>>) {
        let mut buffer = [0; MAX_MESSAGE_SIZE];

        loop {
            match stream.read(&mut buffer) {
                Ok(bytes_read) => {
                    if bytes_read == 0 {
                        shared_data.lock().unwrap().server_disconnected();
                        break;
                    }

                    let mut shared_data = shared_data.lock().unwrap();
                    match bincode::deserialize(&buffer[..bytes_read]) {
                        Ok(packet) => match packet {
                            InternalServerPacket::ConnectResponse(client_id) => {
                                shared_data.id = Some(client_id);
                                shared_data
                                    .packets
                                    .push_back(ServerPacket::ConnectedSuccessfully);
                            }
                            InternalServerPacket::NewClientConnected(client_id) => shared_data
                                .packets
                                .push_back(ServerPacket::NewClientConnected(client_id)),
                            InternalServerPacket::ClientDisconnected(client_id) => shared_data
                                .packets
                                .push_back(ServerPacket::ClientDisconnected(client_id)),
                            InternalServerPacket::ClientKicked(client_id) => shared_data
                                .packets
                                .push_back(ServerPacket::ClientKicked(client_id)),
                            InternalServerPacket::YouWereKicked => {
                                shared_data.packets.push_back(ServerPacket::YouWereKicked)
                            }
                            InternalServerPacket::ClientToClient(client_id, message) => shared_data
                                .packets
                                .push_back(ServerPacket::ClientToClientMessage(client_id, message)),
                            InternalServerPacket::ServerToClient(message) => shared_data
                                .packets
                                .push_back(ServerPacket::ServerToClientMessage(message)),
                            InternalServerPacket::Ping => {
                                if let Err(e) =
                                    shared_data.send_packet(&InternalClientPacket::PingRespone)
                                {
                                    shared_data.error_log.push(e);
                                }
                            }
                        },
                        Err(e) => shared_data.error_log.push(Error::DeserializePacket {
                            bincode_error: e.to_string(),
                        }),
                    }
                }
                Err(e) => {
                    match e.kind() {
                        io::ErrorKind::ConnectionAborted | io::ErrorKind::ConnectionReset => {
                            shared_data.lock().unwrap().server_disconnected();
                            break;
                        }
                        io::ErrorKind::ConnectionRefused => {
                            shared_data.lock().unwrap().server_refused();
                            break;
                        }
                        _ => shared_data
                            .lock()
                            .unwrap()
                            .error_log
                            .push(Error::ReadError { io_error: e }),
                    }
                    break;
                }
            }
        }
    }
}
