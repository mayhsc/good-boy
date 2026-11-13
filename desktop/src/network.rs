use std::{
    io, net::{TcpListener, TcpStream}, sync::mpsc, thread,
};

use gb_core::serial::link::TcpLink;

pub fn start_host(port: u16) -> mpsc::Receiver<io::Result<TcpLink>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = TcpListener::bind(("0.0.0.0", 7777))
            .and_then(|l| l.accept())
            .map(|(stream, _)| TcpLink::new(stream));
        let _ = tx.send(result);
    });
    rx
}

pub fn start_join(addr: String) -> mpsc::Receiver<io::Result<TcpLink>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(TcpStream::connect(addr).map(TcpLink::new));
    });
    rx
}