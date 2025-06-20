use rand::Rng;
use rand::distr::{Distribution, StandardUniform};
use std::collections::HashMap;
use std::fmt::Display;
use std::hash::Hash;
use std::sync::{Arc, RwLock};
use std::thread::sleep;
use std::time::Duration;

const N_THREADS: usize = 5;
const N_KEYS: i32 = 3;

pub struct Cache<K, V> {
    map: RwLock<HashMap<K, Arc<V>>>,
}

impl<K: Display + Clone + Eq + PartialEq + Hash, V: Clone + Display> Cache<K, V> {
    pub fn new() -> Arc<Self> {
        Arc::new(Cache {
            map: RwLock::new(HashMap::new()),
        })
    }

    pub fn get(
        &self,
        i: usize, /*added for debugging only*/
        k: K,
        func: impl Fn(K) -> V,
    ) -> Arc<V> {
        let read_lock = self.map.read().unwrap();

        return match read_lock.get(&k) {
            Some(value) => {
                println!("thread #{i} scopre che la chiave {k} esiste già -> ritorno");
                value.clone()
            }
            None => {
                println!("thread #{i} scopre che la chiave {k} NON esiste...");
                drop(read_lock);

                let mut write_lock = self.map.write().unwrap();
                //check if while the thread was waiting to obtain the write permissions
                //another thread has already written
                if write_lock.get(&k).is_some() {
                    println!(
                        "thread #{i} scopre che la chiave E' STATA INSERITA MENTRE ASPETTAVA DI INSERIRLA {k} -> ritorno"
                    );
                    write_lock.get(&k).unwrap().clone() //by cloning the Arc just the reference is cloned
                } else {
                    let val = Arc::new(func(k.clone())); //executed inside lock => avoids calling func more than once per key
                    println!("thread {i} inserisce il valore di f({k})={val}");
                    write_lock.insert(k.clone(), val.clone());
                    val
                }
            }
        };
    }
}

pub fn f<K: Display, V: Display>(k: K) -> V
where
    StandardUniform: Distribution<V>,
{
    println!(" > Sono dentro la funzione con la chiave...{k}");
    sleep(Duration::from_secs(2));
    let mut rng = rand::rng();
    let val = rng.random();
    println!(" > la funzione restituisce {k} => {val}");
    val
}

pub fn test() {
    let cache = Cache::<i32, i32>::new();
    let mut vt = Vec::new();

    for i in 0..N_THREADS {
        vt.push(std::thread::spawn({
            let c = cache.clone();
            move || {
                for _ in 0..N_KEYS {
                    let j = rand::rng().random_range(0..N_KEYS);
                    let rng: u64 = rand::rng().random_range(0..5);
                    sleep(Duration::from_secs(rng));
                    println!("thread #{i} con chiave {j} sta per entrare nella get");
                    let ret = c.get(i, j, f);
                    println!("thread #{i} per la chiave {j} restituisce {:?}\n", ret);
                }
            }
        }));
    }
    for v in vt {
        v.join().unwrap();
    }
}
