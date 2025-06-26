mod http;
mod thread_pool;

use clap::Parser;
use std::net::TcpListener;
use std::sync::OnceLock;
use std::{io::BufReader, net::TcpStream};

use crate::http::request::Request;
use crate::thread_pool::ThreadPool;

use std::path::Path;

#[derive(Parser)]
pub struct Args {
    #[arg(long)]
    directory: Option<Box<Path>>,
}

pub static DIRECTORY: OnceLock<Box<Path>> = OnceLock::new();

fn main() {
    env_logger::init();
    let args = Args::parse();

    if let Some(dir) = args.directory {
        DIRECTORY.get_or_init(|| dir);
    }

    log::info!("Logs from your program will appear here!");
    let pool = ThreadPool::auto(5);

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                pool.execute(|| handle_connection(stream));
            }
            Err(e) => {
                log::error!("connection failed: {e}");
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

    match response.write_to(&mut stream) {
        Err(io_error) => {
            log::error!("{io_error}");
        }
        Ok(()) => {
            log::trace!("response sent");
        }
    }
}
