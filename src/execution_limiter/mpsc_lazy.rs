use std::any::Any;
use std::panic::{UnwindSafe, catch_unwind};
use std::sync::mpsc::{Sender, channel};
use std::thread::{JoinHandle, spawn};

type ExecutionStart = ();
type ExecutionRequest = Sender<ExecutionStart>;
type ExecutionDone = ();

pub struct Limiter {
    done_snd: Option<Sender<ExecutionDone>>,
    request_snd: Option<Sender<ExecutionRequest>>,
    thread: Option<JoinHandle<()>>,
}

impl Limiter {
    pub fn new(mut n: usize) -> Self {
        let (done_snd, done_rx) = channel::<ExecutionDone>();
        let (request_snd, request_rx) = channel::<ExecutionRequest>();
        Self {
            done_snd: Some(done_snd),
            request_snd: Some(request_snd),
            thread: Some(spawn(move || {
                while let Ok(snd) = request_rx.recv() {
                    if n > 0 {
                        let _ = snd.send(() as ExecutionStart);
                        n = n - 1;
                    } else {
                        if let Ok(_) = done_rx.recv() {
                            snd.send(() as ExecutionStart).unwrap();
                        }
                    }
                }
            })),
        }
    }

    pub fn execute<T, R>(&self, slow_fn: T) -> Result<R, Box<dyn Any + Send + 'static>>
    where
        T: FnOnce() -> R + UnwindSafe,
        R: Send,
    {
        let (start_snd, start_rx) = channel::<ExecutionStart>();
        let _ = self
            .request_snd
            .as_ref()
            .unwrap()
            .send(start_snd as ExecutionRequest);
        start_rx.recv().unwrap();
        let result = catch_unwind(slow_fn);
        let _ = self.done_snd.as_ref().unwrap().send(() as ExecutionDone);
        result
    }
}

impl Drop for Limiter {
    fn drop(&mut self) {
        self.request_snd.take();
        self.done_snd.take();
        self.thread.take().unwrap().join().unwrap();
    }
}
