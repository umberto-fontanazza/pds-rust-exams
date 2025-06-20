use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::{JoinHandle, spawn};

enum MessageOrSubscribe<M> {
    Message(M),
    Subscribe(Sender<M>),
}

pub struct Subscription<M>(Receiver<M>);

impl<M> Subscription<M> {
    pub fn read(&self) -> Option<M> {
        self.0.recv().ok()
    }
}

pub struct Dispatcher<M>
where
    M: Clone,
{
    sender: Option<Sender<MessageOrSubscribe<M>>>,
    thread: Option<JoinHandle<()>>,
}

impl<M> Dispatcher<M>
where
    M: Clone + Send + 'static,
{
    pub fn new() -> Self {
        let (snd, rx) = channel::<MessageOrSubscribe<M>>();
        let mut subscriptions = Vec::<Sender<M>>::new();
        Self {
            sender: Some(snd),
            thread: Some(spawn(move || {
                while let Ok(incoming) = rx.recv() {
                    match incoming {
                        MessageOrSubscribe::Subscribe(s) => {
                            subscriptions.push(s);
                        }
                        MessageOrSubscribe::Message(m) => {
                            subscriptions = subscriptions
                                .into_iter()
                                .filter(|s| s.send(m.clone()).is_ok())
                                .collect::<Vec<Sender<M>>>();
                        }
                    }
                }
            })),
        }
    }

    pub fn dispatch(&self, msg: M) {
        self.sender
            .as_ref()
            .unwrap()
            .send(MessageOrSubscribe::Message(msg))
            .unwrap();
    }

    pub fn subscribe(&self) -> Subscription<M> {
        let (snd, rx) = channel::<M>();
        self.sender
            .as_ref()
            .unwrap()
            .send(MessageOrSubscribe::Subscribe(snd))
            .unwrap();
        Subscription(rx)
    }
}

impl<M> Drop for Dispatcher<M>
where
    M: Clone,
{
    fn drop(&mut self) {
        self.sender.take().unwrap();
        self.thread.take().unwrap().join().unwrap();
    }
}
