mod encoding;
mod http;
mod thread_pool;

use crate::{http::request::RequestSource, http::HTTPCarrier, thread_pool::ThreadPool};
use clap::Parser;
use env_logger::{Target, WriteStyle::Always};
use log::{Level::Debug, LevelFilter};
use std::{
    net::{TcpListener, TcpStream},
    path::Path,
    sync::OnceLock,
};

#[derive(Parser)]
pub struct Args {
    #[arg(long)]
    directory: Option<Box<Path>>,
}

pub static DIRECTORY: OnceLock<Box<Path>> = OnceLock::new();

fn main() {
    env_logger::Builder::new()
        .filter_level(LevelFilter::Debug)
        .target(Target::Stdout)
        .format_timestamp(None)
        .write_style(Always)
        .parse_default_env()
        .init();
    dbg!(log::max_level());

    let args = Args::parse();

    if let Some(dir) = args.directory {
        DIRECTORY.get_or_init(|| dir);
    } else {
        log::warn!("DIRECTORY not set!");
    }

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

    loop {
        let response = match stream.read_request() {
            Ok(request) => request.handle(),
            Err(Some(err_response)) => err_response,
            Err(None) => break, // Stream has been closed
        };

        let close_sent = response.closing();

        match stream.respond(response) {
            Err(io_error) => {
                if log::log_enabled!(Debug) {
                    log::debug!("failed to write response to stream: {io_error:?}\n{stream:?}");
                } else {
                    log::info!("failed to write response to stream: {io_error}");
                }
            }
            Ok(()) => {
                log::trace!("response sent");
            }
        }
        if close_sent {
            break
        }
    }
}
