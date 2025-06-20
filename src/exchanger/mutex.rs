use rand::Rng;
use std::fmt::Debug;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::thread::sleep;
use std::time::Duration;

const N_THREADS: usize = 10;

struct Exchanger<T: Debug> {
    values: Mutex<(Option<T>, Option<T>)>,
    cv: Condvar,
}

impl<T: Debug> Exchanger<T> {
    fn new() -> Arc<Self> {
        return Arc::new(Exchanger {
            values: Mutex::new((None, None)),
            cv: Condvar::new(),
        });
    }

    fn exchange(&self, value: T) -> T {
        let value_to_return;
        let mut lock = self.values.lock().unwrap();
        lock = self.cv.wait_while(lock, |l| (*l).1.is_some()).unwrap();

        if (*lock).0.is_none() {
            println!("{:?} is first", value);
            (*lock).0 = Some(value);
            self.cv.notify_one();
            lock = self.cv.wait_while(lock, |l| (*l).1.is_none()).unwrap();
            value_to_return = (*lock).1.take().unwrap(); //take replaces the value inside the option with "None"
            self.cv.notify_one();
        } else {
            println!("{:?} is second", value);
            (*lock).1 = Some(value);
            value_to_return = (*lock).0.take().unwrap();
            self.cv.notify_all();
        }

        return value_to_return;
    }
}
pub fn test() {
    println!("\nwarning: some prints might be out of order\n");
    let exchanger = Exchanger::new();

    let mut vec_join = Vec::new();

    for i in 0..N_THREADS {
        vec_join.push(thread::spawn({
            let e = exchanger.clone();
            move || {
                let time = rand::rng().random_range(0..20);
                sleep(Duration::from_secs(time));
                println!("thread {} began exchanging procedure", i);
                let v = e.exchange(i);
                println!("> thread {} got value {}", i, v);
            }
        }))
    }

    for h in vec_join {
        h.join().unwrap();
    }
}
