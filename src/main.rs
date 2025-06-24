mod http;

use log::{error, trace};
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::{io::BufReader, net::TcpStream, thread};

use crate::http::request::Request;

pub struct ThreadPool {
    #[allow(dead_code)]
    workers: Vec<Worker>,
    sender: Sender<Job>,
}

impl ThreadPool {
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender.send(job).unwrap();
    }
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    pub fn auto(min_size: u8) -> ThreadPool {
        let available = thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1) as u8;
        let pool_size = (available - 1).max(min_size);

        let mut workers = Vec::with_capacity(pool_size as usize);
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        for id in 0..pool_size {
            workers.push(new_worker(id, Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender }
    }
}

type Worker = thread::JoinHandle<()>;

fn new_worker(id: u8, receiver: Arc<Mutex<Receiver<Job>>>) -> Worker {
    let thread = thread::spawn(move || loop {
        let job = receiver.lock().unwrap().recv().unwrap();
        trace!("worker {id} got a job; executing");
        job();
    });

    thread
}

fn main() {
    env_logger::init();
    log::info!("Logs from your program will appear here!");
    let pool = ThreadPool::auto(5);

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                pool.execute(|| handle_connection(stream));
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

    match response.write_to(&mut stream) {
        Err(io_error) => {
            error!("{io_error}");
        }
        Ok(_) => {
            trace!("response sent");
        }
    }
}
