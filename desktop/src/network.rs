use std::{
    io,
    net::{TcpListener, TcpStream},
};

use gb_core::serial::link::TcpLink;

pub fn host() -> io::Result<TcpLink> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    println!("Listening on {}", listener.local_addr()?);
    let (stream, addr) = listener.accept()?;
    println!("Client connected: {}", addr);

    Ok(TcpLink::new(stream))
}

pub fn join(port: u16) -> io::Result<TcpLink> {
    let stream = TcpStream::connect(format!("127.0.0.1:{port}"))?;
    println!("Connected to host");

    Ok(TcpLink::new(stream))
}
