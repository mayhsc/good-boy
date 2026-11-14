use std::{
    io,
    net::{TcpListener, TcpStream},
    sync::mpsc,
    thread,
};

use gb_core::serial::link::TcpLink;

pub fn start_host() -> io::Result<(u16, mpsc::Receiver<io::Result<TcpLink>>)> {
    let listener = TcpListener::bind(("0.0.0.0", 0))?;
    let port = listener.local_addr()?.port();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = listener.accept().map(|(stream, _)| TcpLink::new(stream));
        let _ = tx.send(result);
    });
    Ok((port, rx))
}

pub fn start_join(addr: String) -> mpsc::Receiver<io::Result<TcpLink>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(TcpStream::connect(addr).map(TcpLink::new));
    });
    rx
}
