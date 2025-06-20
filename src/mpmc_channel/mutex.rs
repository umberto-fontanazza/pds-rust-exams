use ChannelState::{Closed, Open};
use rand::Rng;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::thread::sleep;
use std::time::Duration;

#[derive(PartialEq)]
enum ChannelState {
    Open,
    Closed,
}

pub struct MpMcChannel<E: Send> {
    c_buffer: Mutex<(ChannelState, Vec<E>)>, //only issue is linear access time to extract order element (all must be shifted): use VecDeque<E> for improved performance
    buffer_size: usize,
    cv: Condvar,
}

impl<E: Send> MpMcChannel<E> {
    pub fn new(n: usize) -> Self {
        return MpMcChannel {
            c_buffer: Mutex::new((Open, Vec::with_capacity(n))),
            buffer_size: n,
            cv: Condvar::new(),
        };
    }

    pub fn send(&self, e: E) -> Option<()> {
        let mut try_lock = self.c_buffer.lock();
        if try_lock.is_err() {
            return None;
        }
        let mut lock = try_lock.unwrap();

        if (*lock).1.len() == self.buffer_size {
            println!("Buffer is full, sending can't be fulfilled, thread is waiting...")
        }
        try_lock = self
            .cv
            .wait_while(lock, |l| (*l).1.len() == self.buffer_size && (*l).0 == Open);
        if try_lock.is_err() {
            return None;
        }
        lock = try_lock.unwrap();

        if (*lock).0 == Closed {
            return None;
        }
        (*lock).1.push(e);
        println!("Pushing... Buffer size = {}", (*lock).1.len());
        self.cv.notify_all();

        return Some(());
    }

    pub fn recv(&self) -> Option<E> {
        let mut try_lock = self.c_buffer.lock();
        if try_lock.is_err() {
            return None;
        }
        let mut lock = try_lock.unwrap();

        if (*lock).1.len() == 0 {
            println!("Buffer is empty")
        }
        try_lock = self
            .cv
            .wait_while(lock, |l| (*l).1.len() == 0 && (*l).0 == Open);
        if try_lock.is_err() {
            return None;
        }
        lock = try_lock.unwrap();

        if (*lock).0 == Closed && (*lock).1.len() == 0 {
            return None;
        }
        let e = (*lock).1.remove(0); //first
        println!("Popping (from head)... Buffer size = {}", (*lock).1.len());
        self.cv.notify_all();

        return Some(e);
    }

    pub fn shutdown(&self) -> Option<()> {
        let try_lock = self.c_buffer.lock();
        if try_lock.is_err() {
            return None;
        }
        let mut lock = try_lock.unwrap();

        if (*lock).0 == Closed {
            return None;
        }
        (*lock).0 = Closed;
        return Some(());
    }
}

pub fn test() {
    println!("Please note that the print might not be perfectly synchronized");
    println!(
        "As a matter of fact, in order to grant full synchronization print should be moved when thread has lock"
    );
    println!("Inside both the send() and the return() function\n\n");

    let channel = Arc::new(MpMcChannel::new(5));

    let mut handles = vec![];

    for i in 0..4 {
        handles.push(thread::spawn({
            let channel = channel.clone();
            move || {
                for j in 0..8 {
                    if i < 2 {
                        let time = rand::rng().random_range(0..1);
                        sleep(Duration::from_secs(time));
                        println!("thread {} sending {}", i, j);
                        channel.send((i, j));
                        if j == 8 {
                            println!("shutting down...");
                            channel.shutdown();
                        }
                    } else {
                        let time = rand::rng().random_range(2..3);
                        sleep(Duration::from_secs(time));
                        let e = channel.recv();
                        println!(
                            "thread {} received from thread {} value {}",
                            i,
                            e.unwrap().0,
                            e.unwrap().1
                        );
                    }
                }
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}
