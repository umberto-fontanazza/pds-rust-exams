use rand::Rng;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::thread::sleep;
use std::time::Duration;

const N_THREADS: usize = 10;

pub struct CountDownLatch {
    cv: Condvar,
    counter: Mutex<usize>,
}

impl CountDownLatch {
    pub fn new(counter: usize) -> Arc<Self> {
        return Arc::new(CountDownLatch {
            cv: Condvar::new(),
            counter: Mutex::new(counter),
        });
    }

    pub fn awaiting(&self, index: usize) {
        let lock = self.counter.lock().unwrap();
        println!("Thread {} is waiting...", index);
        drop(self.cv.wait_while(lock, |l| *l > 0).unwrap());
        println!("Thread {} returns!", index);
    }

    pub fn count_down(&self, index: usize) {
        println!("Thread {} counting down...", index);
        let mut lock = self.counter.lock().unwrap();

        if *lock == 0 {
            return;
        }

        *lock -= 1;
        println!("Thread {} decreased from {} to {}", index, *lock + 1, *lock);
        if *lock == 0 {
            self.cv.notify_all()
        }
    }
}

pub fn test() {
    let latch = CountDownLatch::new(N_THREADS / 2);

    let mut handles = vec![];

    for i in 0..N_THREADS {
        handles.push(thread::spawn({
            let latch = latch.clone();
            move || {
                let time = rand::rng().random_range(3..10);
                sleep(Duration::from_secs((time - i % 2 * 2) as u64));
                if i % 2 == 0 {
                    latch.awaiting(i);
                } else {
                    latch.count_down(i);
                }
            }
        }))
    }

    for h in handles {
        h.join().unwrap();
    }
}
