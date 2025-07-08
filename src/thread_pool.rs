use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

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
        let available: u8 = thread::available_parallelism()
            .map_or(1, usize::from)
            .try_into()
            .unwrap_or(1);
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
        log::info!("worker {id} got a job; executing");
        job();
    });

    thread
}
