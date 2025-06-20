use std::cmp::Ordering;
use std::cmp::min;
use std::collections::{BinaryHeap, VecDeque};
use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::thread::{JoinHandle, spawn};
use std::time::{Duration, Instant};

const ONE_WEEK: Duration = Duration::from_secs(60 * 60 * 24 * 7);

struct Timed<T>(Instant, T);

impl<T> PartialEq for Timed<T> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl<T> Eq for Timed<T> {}

impl<T> PartialOrd for Timed<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Timed<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0).reverse()
    }
}

enum Op<T: Send> {
    Offer(Instant, T),
    Take(Sender<Option<T>>),
    Length(Sender<usize>),
}

pub struct DelayedQueue<T: Send> {
    op_sender: Option<Sender<Op<T>>>,
    thread: Option<JoinHandle<()>>,
}

impl<T> DelayedQueue<T>
where
    T: Send + 'static,
{
    pub fn new() -> Self {
        let (op_sender, op_rx) = channel::<Op<T>>();
        let op_sender = Some(op_sender);
        Self {
            op_sender,
            thread: Some(spawn(move || {
                let mut take_queue = VecDeque::<Sender<Option<T>>>::new();
                let mut ready_items = VecDeque::<T>::new();
                let mut timed_items = BinaryHeap::<Timed<T>>::new();
                let mut next_check = Instant::now() + ONE_WEEK;
                loop {
                    match op_rx.recv_timeout(next_check - Instant::now()) {
                        Ok(Op::Offer(t, item)) => {
                            if t < Instant::now() && take_queue.len() > 0 {
                                take_queue.pop_front().unwrap().send(Some(item)).unwrap();
                            } else if t < Instant::now() && take_queue.len() == 0 {
                                ready_items.push_back(item);
                            } else {
                                next_check = min(next_check, t);
                                timed_items.push(Timed(t, item));
                            }
                        }
                        Ok(Op::Take(item_snd)) => {
                            if ready_items.len() > 0 {
                                item_snd
                                    .send(Some(ready_items.pop_front().unwrap()))
                                    .unwrap();
                            } else if timed_items.len() - take_queue.len() > 0 {
                                take_queue.push_back(item_snd);
                            } else {
                                item_snd.send(None).unwrap();
                            }
                        }
                        Ok(Op::Length(len_snd)) => {
                            len_snd
                                .send(ready_items.len() + timed_items.len() - take_queue.len())
                                .unwrap();
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            if timed_items.len() == 0 {
                                next_check += ONE_WEEK;
                                return;
                            }
                            let Timed(_, item) = timed_items.pop().unwrap();
                            if take_queue.len() > 0 {
                                take_queue.pop_front().unwrap().send(Some(item)).unwrap();
                            } else {
                                ready_items.push_back(item);
                            }
                        }
                        Err(RecvTimeoutError::Disconnected) => {
                            break;
                        }
                    }
                }
            })),
        }
    }

    pub fn offer(&self, item: T, t: Instant) {
        self.op_sender
            .as_ref()
            .unwrap()
            .send(Op::Offer(t, item))
            .unwrap();
    }

    pub fn take(&self) -> Option<T> {
        let (res_snd, res_rx) = channel::<Option<T>>();
        self.op_sender
            .as_ref()
            .unwrap()
            .send(Op::Take(res_snd))
            .unwrap();
        res_rx.recv().unwrap()
    }

    pub fn size(&self) -> usize {
        let (len_snd, len_rx) = channel::<usize>();
        self.op_sender
            .as_ref()
            .unwrap()
            .send(Op::Length(len_snd))
            .unwrap();
        len_rx.recv().unwrap()
    }
}

impl<T> Drop for DelayedQueue<T>
where
    T: Send,
{
    fn drop(&mut self) {
        self.op_sender.take();
        self.thread.take().unwrap().join().unwrap();
    }
}
