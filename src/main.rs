#[allow(unused_imports)]
use std::net::TcpListener;
use std::{io::Write, net::TcpStream};

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    log::info!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_connection(stream);
            }
            Err(e) => {
                log::error!("connection failed: {}", e);
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    log::info!("accepted new connection");
    stream
        .write_all("HTTP/1.1 200 OK\r\n\r\n".as_bytes())
        .unwrap_or_else(|e| eprintln!("{e}"));
}
