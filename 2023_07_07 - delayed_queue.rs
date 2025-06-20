/*
2023-07-07
    Una DelayedQueue<T:Send> è un particolare tipo di coda non limitata che offre tre metodi
    principali, oltre alla funzione costruttrice:
        1. offer(&self, t:T, i: Instant) : Inserisce un elemento che non potrà essere estratto prima
           dell'istante di scadenza i.
        2. take(&self) -> Option<T>: Cerca l'elemento t con scadenza più ravvicinata: se tale
           scadenza è già stata oltrepassata, restituisce Some(t); se la scadenza non è ancora stata
           superata, attende senza consumare cicli di CPU, che tale tempo trascorra, per poi restituire
           Some(t); se non è presente nessun elemento in coda, restituisce None. Se, durante l'attesa,
           avviene un cambiamento qualsiasi al contenuto della coda, ripete il procedimento suddetto
           con il nuovo elemento a scadenza più ravvicinata (ammesso che ci sia ancora).
        3. size(&self) -> usize: restituisce il numero di elementi in coda indipendentemente dal fatto
           che siano scaduti o meno.
    Si implementi tale struttura dati nel linguaggio Rust, avendo cura di renderne il comportamento
    thread-safe. Si ricordi che gli oggetti di tipo Condvar offrono un meccanismo di attesa limitata nel
    tempo, offerto dai metodi wait_timeout(...) e wait_timeout_while(...)).
*/

use rand::Rng;
use std::cmp::Ordering;
use std::fmt::Debug;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const N_THREADS: usize = 24;

#[derive(PartialEq, Clone, Copy, Debug)]
enum TimeComparison {
    Lesser,
    Equal,
    Greater,
}

fn time_difference(i1: Instant, i2: Instant) -> (Duration, TimeComparison) {
    return match i1.cmp(&i2) {
        Ordering::Less => (i2.duration_since(i1), TimeComparison::Lesser),
        Ordering::Equal => (i2.duration_since(i1), TimeComparison::Equal),
        Ordering::Greater => (i1.duration_since(i2), TimeComparison::Greater),
    };
}

struct DelayedQueue<T: Send + Clone + Debug + PartialEq> {
    queue: Mutex<Vec<(Instant, T)>>,
    cv: Condvar,
}

impl<T: Send + Clone + Debug + PartialEq> DelayedQueue<T> {
    fn new() -> Self {
        DelayedQueue {
            queue: Mutex::new(Vec::new()),
            cv: Condvar::new(),
        }
    }

    fn offer(&self, t: T, i: Instant) {
        self.queue.lock().unwrap().push((i, t));
        self.cv.notify_all();
    }

    //the exercise is solved by supposing that the function extracts the seeked element from the queue
    fn take(&self) -> Option<T> {
        loop {
            let mut lock = self.queue.lock().unwrap();
            let queue_length = lock.len();
            let mut nearest_difference: Option<(Duration, TimeComparison)> = None;
            let mut nearest_element = None;
            let mut nearest_pos = None;
            let now = Instant::now();

            if lock.len() == 0 {
                break;
            }

            let mut pos = 0;
            for element in lock.iter() {
                let current_difference = time_difference(element.0, now);
                if nearest_element.is_none() || current_difference.0 < nearest_difference.unwrap().0
                {
                    nearest_difference = Some(current_difference);
                    nearest_element = Some(element.clone());
                    nearest_pos = Some(pos);
                }
                pos += 1;
            }

            if nearest_difference.unwrap().1 == TimeComparison::Greater {
                lock = self
                    .cv
                    .wait_timeout_while(lock, nearest_difference.unwrap().0, |l| {
                        (*l).len() == queue_length
                    })
                    .unwrap()
                    .0;
                if lock.len() == queue_length &&                                                               //  check if queue
                    *(lock.iter().nth(nearest_pos.clone().unwrap()).unwrap()) == nearest_element.unwrap()
                //  changed (in case, run again)
                {
                    return Some(lock.remove(nearest_pos.unwrap()).1);
                }
            } else {
                let res = Some(lock.remove(nearest_pos.unwrap()).1);
                self.cv.notify_all();
                return res;
            }
        } //loop
        return None;
    }

    fn size(&self) -> usize {
        self.queue.lock().unwrap().len()
    }
}

fn main() {
    let delayed_queue = Arc::new(DelayedQueue::<usize>::new());

    let mut thread_handles = Vec::new();

    for i in 0..N_THREADS {
        thread_handles.push(thread::spawn({
            let d = delayed_queue.clone();
            move || {
                if i > N_THREADS * 3 / 4 {
                    thread::sleep(Duration::from_secs(rand::thread_rng().gen_range(0..15)));
                    let instant = Instant::now();
                    println!("> Thread {i} taking val from queue...");
                    let val = d.take();
                    println!("> Thread {i} took val {:?} at instant {:?}", val, instant);
                } else {
                    thread::sleep(Duration::from_secs(rand::thread_rng().gen_range(2..20)));
                    let instant =
                        Instant::now() + Duration::from_secs(rand::thread_rng().gen_range(1..10));
                    d.offer(i, instant);
                    println!("pushing {},{:?} into the queue", i, instant);
                }
            }
        }))
    }

    println!("Final size of the queue = {}", delayed_queue.size());
    //println!("Delayed queue = {:?}", delayed_queue);

    for h in thread_handles {
        h.join().unwrap();
    }
}
