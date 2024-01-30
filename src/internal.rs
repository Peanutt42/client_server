use serde::{Serialize, Deserialize};
use crate::{ClientId, RawMessageData};

#[derive(Serialize, Deserialize)]
pub enum InternalClientPacket {
    /// packet gets sent to all other clients
    BroadcastMessage(RawMessageData),
    /// (client to send to, message)
    /// packet gets sent to the specific client
    PersonalMessage(ClientId, RawMessageData),
    /// packet only gets sent to the server
    ServerMessage(RawMessageData),
    PingRespone,
}

/// These are actually sent over the network
#[derive(Serialize, Deserialize)]
pub enum InternalServerPacket {
    ConnectResponse(ClientId),
    NewClientConnected(ClientId),
    ClientDisconnected(ClientId),
    ClientKicked(ClientId),
    YouWereKicked,
    /// author, message
    ClientToClient(ClientId, RawMessageData),
    ServerToClient(RawMessageData),
    Ping,
}