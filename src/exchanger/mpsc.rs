use std::sync::{
    Arc,
    mpsc::{Sender, channel},
};
use std::thread::{JoinHandle, spawn};

pub struct Exchanger<T: Send> {
    sender: Option<Sender<(T, Sender<T>)>>,
    thread: Option<JoinHandle<()>>,
}

impl<T: Send + 'static> Exchanger<T> {
    pub fn new() -> Self {
        let (sender, rx) = channel::<(T, Sender<T>)>();
        let sender = Some(sender);
        Self {
            sender,
            thread: Some(spawn(move || {
                let mut first = None as Option<(T, Sender<T>)>;
                while let Ok((item, item_snd)) = rx.recv() {
                    match first.take() {
                        Some((first_item, first_snd)) => {
                            first_snd.send(item).unwrap();
                            item_snd.send(first_item).unwrap();
                        }
                        None => {
                            first = Some((item, item_snd));
                        }
                    }
                }
            })),
        }
    }
    pub fn exchange(&self, item: T) -> T {
        let (snd, rx) = channel::<T>();
        self.sender.as_ref().unwrap().send((item, snd)).unwrap();
        rx.recv().unwrap()
    }
}

impl<T: Send> Drop for Exchanger<T> {
    fn drop(&mut self) {
        self.sender.take().unwrap();
        self.thread.take().unwrap().join().unwrap();
    }
}

pub fn test() {
    let e = Arc::new(Exchanger::<usize>::new());
    let e2 = e.clone();

    let _ = spawn(move || {
        e2.exchange(10);
    });
    let res = e.exchange(1);
    println!("{res}");
}
