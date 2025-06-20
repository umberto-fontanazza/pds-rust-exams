use std::sync::{Arc, Condvar, Mutex};

const N: usize = 5;

#[derive(PartialEq)]
enum State {
    Progress,
    Closure,
}

struct RankingBarrier {
    n_threads: usize, //no need to protect here because it is wrapped inside an Arc and is never written
    counter: Mutex<(usize, State)>,
    cv: Condvar,
}

impl RankingBarrier {
    fn new(n_threads: usize) -> Result<Arc<Self>, ()> {
        match n_threads {
            0..=1 => Err(()),
            _ => Ok(Arc::new(RankingBarrier {
                n_threads,
                counter: Mutex::new((0, State::Progress)),
                cv: Condvar::new(),
            })),
        }
    }

    pub fn wait(&self, thread_index: usize /*only added for clarity*/) -> usize {
        let mut lock = self.counter.lock().unwrap();

        lock = self
            .cv
            .wait_while(lock, |l| (*l).1 == State::Progress && (*l).0 > 0)
            .unwrap();

        //arrives here only if state is progress and counter is 0 (everybody out)
        if (*lock).1 == State::Progress {
            (*lock).1 = State::Closure;
            println!();
            self.cv.notify_all();
        }
        (*lock).0 += 1;

        let ret = (*lock).0;

        println!("Thread {thread_index} comes {}th", ret);

        while (*lock).0 != self.n_threads && (*lock).1 == State::Closure {
            lock = self.cv.wait(lock).unwrap();
        }

        if (*lock).1 == State::Closure {
            (*lock).1 = State::Progress;
            println!();
            self.cv.notify_all();
        }

        (*lock).0 -= 1;

        println!("Thread {thread_index} returning {}", ret);
        return ret;
    }
}

pub fn test() {
    let c_barrier = RankingBarrier::new(N)
        .expect("At least 2 threads are required for the barrier to work properly");
    let mut vt = Vec::new();

    for i in 0..N {
        vt.push(std::thread::spawn({
            let c = c_barrier.clone();
            move || {
                for _ in 0..3 {
                    c.wait(i);
                }
            }
        }));
    }

    for t in vt {
        t.join().unwrap();
    }
}
