use std::sync::Arc;
use std::sync::mpsc::{Sender, channel};
use std::thread::{JoinHandle, spawn};

pub struct Looper<T: Send> {
    sender: Option<Sender<T>>,
    thread: Option<JoinHandle<()>>,
}

impl<T: Send + 'static> Looper<T> {
    pub fn new<P, C>(mut process: P, cleanup: C) -> Self
    where
        P: FnMut(T) -> () + Send + 'static,
        C: FnOnce() -> () + Send + 'static,
    {
        let (sender, receiver) = channel::<T>();
        let thread = spawn(move || {
            while let Ok(message) = receiver.recv() {
                process(message)
            }
            cleanup();
        });
        let sender = Some(sender);
        let thread = Some(thread);
        Self { sender, thread }
    }

    pub fn send(&self, message: T) {
        self.sender.as_ref().unwrap().send(message).unwrap();
    }
}

impl<T: Send> Drop for Looper<T> {
    fn drop(&mut self) {
        self.sender.take().unwrap();
        self.thread.take().unwrap().join().unwrap();
    }
}

pub fn test() {
    let process = |number: usize| {
        println!("Thread {number} sent to the Looper!");
    };
    let cleanup = || {
        println!("Cleaning up");
    };
    let my_looper = Arc::new(Looper::new(process, cleanup));

    let (count_threads, count_messages) = (5, 5);
    let mut threads = Vec::<JoinHandle<()>>::with_capacity(count_threads);
    for thread_id in 0..count_threads {
        let looper_arc = my_looper.clone();
        threads.push(spawn(move || {
            (0..count_messages).for_each(|_| {
                looper_arc.send(thread_id);
            });
        }));
    }
    threads
        .into_iter()
        .for_each(|handle| handle.join().unwrap());
}
