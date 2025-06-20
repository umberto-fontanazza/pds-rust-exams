use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

#[derive(PartialEq)]
enum DispatcherState {
    Connected,
    Disconnected,
}

pub struct Dispatcher<M>
where
    M: Send + Clone,
{
    mutex: Mutex<Vec<Arc<Subscription<M>>>>,
}

impl<M> Dispatcher<M>
where
    M: Send + Clone,
{
    pub fn new() -> Self {
        Self {
            mutex: Mutex::new(Vec::<Arc<Subscription<M>>>::new()),
        }
    }

    pub fn dispatch(&self, message: M) {
        let lock = self.mutex.lock().unwrap();
        lock.iter()
            .zip((0..(lock.len())).into_iter().map(|_| message.clone()))
            .for_each(|(subscription_arc, m_clone)| {
                subscription_arc.dispatch(m_clone);
            });
    }

    pub fn subscribe(&self) -> Arc<Subscription<M>> {
        let sub = Arc::new(Subscription::new());
        let mut lock = self.mutex.lock().unwrap();
        lock.push(sub.clone());
        sub
    }
}

impl<M> Drop for Dispatcher<M>
where
    M: Send + Clone,
{
    fn drop(&mut self) {
        let lock = self.mutex.lock().unwrap();
        lock.iter()
            .for_each(|subscription| subscription.disconnect());
    }
}

pub struct Subscription<M>
where
    M: Send + Clone,
{
    mutex: Mutex<(VecDeque<M>, DispatcherState)>,
    condvar: Condvar,
}

impl<M> Subscription<M>
where
    M: Send + Clone,
{
    pub fn new() -> Self {
        Self {
            mutex: Mutex::new((VecDeque::<M>::new(), DispatcherState::Connected)),
            condvar: Condvar::new(),
        }
    }

    pub fn read(&self) -> Option<M> {
        let mut lock = self.mutex.lock().unwrap();
        lock = self
            .condvar
            .wait_while(lock, |l| {
                l.0.len() == 0 && l.1 == DispatcherState::Connected
            })
            .unwrap();
        lock.0.pop_front()
    }

    fn dispatch(&self, message: M) {
        let mut lock = self.mutex.lock().unwrap();
        lock.0.push_back(message);
        self.condvar.notify_one();
    }

    fn disconnect(&self) {
        let mut lock = self.mutex.lock().unwrap();
        lock.1 = DispatcherState::Disconnected;
        self.condvar.notify_all();
    }
}
