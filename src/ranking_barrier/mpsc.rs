use std::sync::mpsc::{Sender, channel};
use std::thread::{JoinHandle, spawn};

type WaitRequest = Sender<usize>;

pub struct RankingBarrier {
    sender: Option<Sender<WaitRequest>>,
    thread: Option<JoinHandle<()>>,
}

impl RankingBarrier {
    pub fn new(n: usize) -> Result<Self, ()> {
        if n < 2 {
            return Err(());
        }
        let (snd, rx) = channel::<WaitRequest>();
        let sender = Some(snd);

        Ok(Self {
            sender,
            thread: Some(spawn(move || {
                let mut queue = Vec::<Sender<usize>>::with_capacity(n);
                while let Ok(snd_rank) = rx.recv() {
                    queue.push(snd_rank);
                    if queue.len() == n {
                        queue
                            .iter()
                            .enumerate()
                            .for_each(|(index, snd_rank)| snd_rank.send(index + 1).unwrap());
                        queue.clear();
                    }
                }
            })),
        })
    }

    pub fn wait(&self) {
        let (snd_rank, rx_rank) = channel::<usize>();
        self.sender.as_ref().unwrap().send(snd_rank).unwrap();
        rx_rank.recv().unwrap();
    }
}

impl Drop for RankingBarrier {
    fn drop(&mut self) {
        self.sender.take().unwrap();
        self.thread.take().unwrap().join().unwrap();
    }
}
