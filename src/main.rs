mod http;

#[allow(unused_imports)]
use std::net::TcpListener;
use std::{
    io::{BufReader, Write},
    net::TcpStream,
};

use log::{error, trace};

use crate::http::request::Request;

fn main() {
    log::info!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_connection(stream);
            }
            Err(e) => {
                error!("connection failed: {}", e);
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    log::info!("accepted new connection");
    let buf_reader = BufReader::new(&mut stream);

    let response = match Request::try_from(buf_reader) {
        Ok(request) => request.make_response(),
        Err(err) => err,
    };

    let buf = &response.serialize();
    trace!("sending response: {}", String::from_utf8_lossy(buf));
    match stream.write_all(buf) {
        Ok(_) => (), // sucessfull write
        Err(io_error) => eprintln!("{io_error}"),
    };
    stream.flush().unwrap();
    trace!("response sent");
}
