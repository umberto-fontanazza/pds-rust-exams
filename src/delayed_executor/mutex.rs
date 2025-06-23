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
    fn wait_do<F>(self, closure: F)
    where
        F: FnOnce(T) -> (),
    {
        if self.time > Instant::now() {
            sleep(self.time - Instant::now());
        }
        closure(self.item)
    }
}

pub struct DelayedExecutor {
    arc: Arc<(Mutex<(BinaryHeap<Timed<Task>>, State)>, Condvar)>,
    executor: Option<JoinHandle<()>>,
}

impl DelayedExecutor {
    pub fn new() -> Self {
        let arc = Arc::new((
            Mutex::new((BinaryHeap::<Timed<Task>>::new(), State::Open)),
            Condvar::new(),
        ));
        let executor_cb = {
            let mut timeout = Duration::MAX;
            let arc = arc.clone();
            move || {
                loop {
                    let mut guard = arc
                        .1
                        .wait_timeout_while(arc.0.lock().unwrap(), timeout, |(heap, state)| {
                            heap.is_empty() && *state == State::Open
                                || heap.peek().is_some_and(|time_task| {
                                    timeout = time_task.time - Instant::now();
                                    !timeout.is_zero()
                                })
                        })
                        .unwrap()
                        .0;
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
                            sorted_tasks
                                .into_iter()
                                .for_each(|timed| timed.wait_do(|callable| callable()));
                            break;
                        }
                    }
                }
            }
        };
        Self {
            arc,
            executor: Some(spawn(executor_cb)),
        }
    }

    pub fn execute_no_delay<F>(&self, task: F) -> bool
    where
        F: FnOnce() + Send + 'static,
    {
        self.execute(task, Duration::from_millis(0))
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
                self.arc.1.notify_one();
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

pub fn test() {
    let start = Instant::now();
    let de = Arc::new(DelayedExecutor::new());
    let task1 = move || println!("I'm task 1 {:?}", Instant::now() - start);
    let task2 = move || println!("I'm task 2 {:?}", Instant::now() - start);
    let task3 = move || println!("I'm task 3 {:?}", Instant::now() - start);
    let thread_1 = {
        let de = de.clone();
        spawn(move || de.execute_no_delay(task2))
    };
    let thread_2 = {
        let de = de.clone();
        spawn(move || {
            de.execute(task3, Duration::from_secs(2));
        })
    };
    de.execute(task1, Duration::from_millis(500));
    thread_1.join().unwrap();
    thread_2.join().unwrap();
}
