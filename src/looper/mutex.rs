use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{JoinHandle, spawn};

enum StopWrap<T> {
    Stop,
    Wrap(T),
}

pub struct Looper<M>
where
    M: Send,
{
    queue: Arc<(Mutex<VecDeque<StopWrap<M>>>, Condvar)>,
    thread: Option<JoinHandle<()>>,
}

impl<M> Looper<M>
where
    M: Send + 'static,
{
    pub fn new<P, C>(mut process: P, cleanup: C) -> Self
    where
        P: FnMut(M) -> () + Send + 'static,
        C: FnOnce() -> () + Send + 'static,
    {
        let queue = Arc::new((Mutex::new(VecDeque::<StopWrap<M>>::new()), Condvar::new()));
        let arc_clone = queue.clone();
        let thread = spawn(move || {
            let mut guard = arc_clone.0.lock().unwrap();
            loop {
                guard = arc_clone.1.wait_while(guard, |g| g.len() == 0).unwrap();
                match guard.pop_front().unwrap() {
                    StopWrap::Wrap(message) => {
                        process(message);
                    }
                    StopWrap::Stop => {
                        break;
                    }
                }
            }
            cleanup();
        });
        Self {
            queue,
            thread: Some(thread),
        }
    }

    pub fn send(&self, message: M) {
        let mut lock = self.queue.0.lock().unwrap();
        lock.push_back(StopWrap::Wrap(message));
        self.queue.1.notify_one();
    }
}

impl<M> Drop for Looper<M>
where
    M: Send,
{
    fn drop(&mut self) {
        {
            let mut guard = self.queue.0.lock().unwrap();
            guard.push_back(StopWrap::Stop);
            self.queue.1.notify_one();
        }
        self.thread.take().unwrap().join().unwrap();
    }
}

pub fn test() {
    let l = Looper::new(
        |n: usize| println!("Processing number {n}"),
        || println!("Cleaning up looper"),
    );
    let l = Arc::new(l);
    let handles = (0..3)
        .into_iter()
        .zip((0..3).into_iter().map(|_| l.clone()))
        .map(|(_, looper)| {
            spawn(move || {
                (0..10).into_iter().for_each(|n| {
                    looper.send(n);
                });
            })
        })
        .collect::<Vec<_>>();
    handles.into_iter().for_each(|handle| {
        handle.join().unwrap();
    });
}
