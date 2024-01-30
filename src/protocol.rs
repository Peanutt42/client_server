use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};

pub enum NetworkingListener {
    Tcp(TcpListener),
}

impl NetworkingListener {
    pub fn set_nonblocking(&mut self, nonblocking: bool) -> io::Result<()> {
        match self {
            Self::Tcp(listener) => listener.set_nonblocking(nonblocking),
        }
    }

    pub fn incoming(&mut self) -> Incoming<'_> {
        Incoming { listener: self }
    }
}

pub enum NetworkingStream {
    Tcp(TcpStream),
}

impl NetworkingStream {
    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Tcp(stream) => stream.read(buf),
        }
    }

    pub fn write(&mut self, buf: &[u8]) -> io::Result<()> {
        match self {
            Self::Tcp(stream) => stream.write_all(buf),
        }
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        match self {
            Self::Tcp(stream) => stream.peer_addr(),
        }
    }

    pub fn try_clone(&self) -> io::Result<Self> {
        match self {
            Self::Tcp(stream) => match stream.try_clone() {
                Ok(cloned_stream) => Ok(Self::Tcp(cloned_stream)),
                Err(e) => Err(e),
            },
        }
    }
}

pub struct Incoming<'a> {
    listener: &'a NetworkingListener,
}

impl<'a> Iterator for Incoming<'a> {
    type Item = io::Result<NetworkingStream>;
    fn next(&mut self) -> Option<io::Result<NetworkingStream>> {
        match self.listener {
            NetworkingListener::Tcp(listener) => Some(
                listener
                    .accept()
                    .map(|(stream, _address)| NetworkingStream::Tcp(stream)),
            ),
        }
    }
}
