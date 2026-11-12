use core::time;
use std::{
    io::{self, Read, Write},
    net::TcpStream,
    time::Duration,
};

pub trait SerialLink: std::fmt::Debug {
    fn is_connected(&self) -> bool;
    fn send(&mut self, byte: u8);
    fn try_recv(&mut self) -> Option<u8>;
    fn recv_timeout(&mut self, timeout: Duration) -> Option<u8>;
}

#[derive(Debug)]
pub struct TcpLink {
    stream: TcpStream,
    connected: bool,
}

impl TcpLink {
    pub fn new(stream: TcpStream) -> Self {
        Self { stream, connected: true }
    }
}

impl SerialLink for TcpLink {
    fn is_connected(&self) -> bool {
        self.connected
    }

    fn send(&mut self, byte: u8) {
        match self.stream.write(&[byte]) {
            Ok(1) => {}
            Ok(_) => self.connected = false,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(_) => self.connected = false,
        }
    }

    fn try_recv(&mut self) -> Option<u8> {
        self.stream.set_nonblocking(true).unwrap();

        let mut buf = [0u8; 1];

        match self.stream.read(&mut buf) {
            Ok(1) => Some(buf[0]),
            Ok(_) => {
                self.connected = false;
                None
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => None,
            Err(_) => {
                self.connected = false;
                None
            }
        }
    }

    fn recv_timeout(&mut self, timeout: Duration) -> Option<u8> {
        self.stream.set_nonblocking(false).unwrap();
        self.stream.set_read_timeout(Some(timeout)).unwrap();

        let mut buf = [0u8; 1];

        match self.stream.read(&mut buf) {
            Ok(1) => Some(buf[0]),
            Ok(_) => {
                self.connected = false;
                None
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => None,
            Err(_) => {
                self.connected = false;
                None
            }
        }
    }
}
