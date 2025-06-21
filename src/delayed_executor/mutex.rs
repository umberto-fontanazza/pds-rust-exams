use std::collections::BinaryHeap;
use std::sync::Condvar;
use std::thread::spawn;
use std::time::Duration;
use std::{
    sync::{Arc, Mutex},
    thread::JoinHandle,
    thread::sleep,
    time::Instant,
};

type Task = Box<dyn FnOnce() -> () + Send + 'static>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Open,
    Closed,
}

struct Timed<T> {
    item: T,
    time: Instant,
}

impl<T> PartialEq for Timed<T> {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl<T> Eq for Timed<T> {}

impl<T> PartialOrd for Timed<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Timed<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time).reverse()
    }
}

impl<T> Timed<T> {
    fn is_ready(&self) -> bool {
        Instant::now() >= self.time
    }

    #[inline]
    fn is_waiting(&self) -> bool {
        !self.is_ready()
    }
}

pub struct DelayedExecutor {
    arc: Arc<(Mutex<(BinaryHeap<Timed<Task>>, State)>, Condvar)>,
    executor: Option<JoinHandle<()>>,
}

impl DelayedExecutor {
    pub fn new() -> Self {
        let arc = Arc::new((Mutex::new((BinaryHeap::new(), State::Open)), Condvar::new()));
        Self {
            arc: arc.clone(),
            executor: Some(spawn(move || {
                loop {
                    let mut guard = arc
                        .1
                        .wait_while(arc.0.lock().unwrap(), |(heap, state)| {
                            heap.is_empty() && *state == State::Open
                                || heap.peek().is_some_and(|time_task| time_task.is_waiting())
                        })
                        .unwrap();
                    let (heap, state) = &mut *guard;
                    match *state {
                        State::Open => {
                            let task: Task = heap.pop().unwrap().item;
                            drop(guard);
                            task()
                        }
                        State::Closed => {
                            let sorted_tasks = (0..heap.len())
                                .map(|_| heap.pop().unwrap())
                                .collect::<Vec<Timed<Task>>>();
                            drop(guard);
                            sorted_tasks.into_iter().for_each(
                                |Timed::<_> {
                                     item: task_fn,
                                     time,
                                 }| {
                                    if time > Instant::now() {
                                        sleep(Instant::now() - time);
                                    }
                                    task_fn()
                                },
                            );
                            break;
                        }
                    }
                }
            })),
        }
    }

    pub fn execute<F>(&self, task: F, delay: Duration) -> bool
    where
        F: FnOnce() + Send + 'static,
    {
        let timed_task = Timed::<Task> {
            item: Box::new(task),
            time: Instant::now() + delay,
        };
        let mut guard = self.arc.0.lock().unwrap();
        let (heap, state) = &mut *guard;
        match state {
            State::Open => {
                heap.push(timed_task);
                true
            }
            State::Closed => false,
        }
    }

    pub fn close(&self, drop_pending_tasks: bool) {
        let mut guard = self.arc.0.lock().unwrap();
        let (heap, state) = &mut *guard;
        if drop_pending_tasks {
            heap.drain();
        }
        *state = State::Closed
    }
}

impl Drop for DelayedExecutor {
    fn drop(&mut self) {
        self.close(false);
        self.executor.take().unwrap().join().unwrap();
    }
}
