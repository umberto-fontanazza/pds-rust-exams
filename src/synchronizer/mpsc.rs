use rand::Rng;
use std::fmt::Debug;
use std::sync::Arc;
use std::sync::mpsc::{SyncSender, sync_channel}; //TODO: since Rust 1.78 the regular Sender is Sync, this might be redundant
use std::thread;
use std::thread::{JoinHandle, sleep};
use std::time::Duration;

const N_READS: usize = 10;

//l'implementazione della porta non è richiesta
struct SerialPort {}

impl SerialPort {
    fn new() -> Self {
        return SerialPort {};
    }

    fn read(&self) -> i32 {
        let time = rand::rng().random_range(0..5);
        sleep(Duration::from_secs(time));
        return rand::rng().random_range(0..5);
    }
}

pub struct Synchronizer<K: Send + 'static + Debug> {
    //quando il synchronizer è instanziato, viene creato un thread aggiuntivo che possiede i capi di entrambi i canali
    //e si occupa di effettuare il processamento di valori letti
    //quando il synchronizer viene istrutto, bisogna preoccuparsi di terminare il thread e aspettarlo
    //l'attesa è fatta tramite l'implementazione del tratto Drop
    processor_handle: Option<JoinHandle<()>>,
    //sync_channel offre uno strumento di sincronizzazione ready-made synchronization tool, il quale non è solo bloccante per il receiver,
    // ma anche per il sender nel caso il buffer (specificato all creazione del canale) sia pieno
    //tale proprietà rende il canale migliore del classico channel per svolgere questo esercizio senza dover utilizzare condvar per rendere i metodi bloccati
    sender1: Option<SyncSender<K>>,
    sender2: Option<SyncSender<K>>,
}

impl<K: Send + 'static + Debug> Synchronizer<K> {
    pub fn new(process: impl Fn(K, K) -> () + Send + 'static) -> Arc<Self> {
        let (s1, r1) = sync_channel(0);
        let (s2, r2) = sync_channel(0);

        let processor_handle = thread::spawn({
            move || {
                let mut res;
                let mut v1;
                let mut v2;
                loop {
                    //quando i canali sono chiusi la ricezione ritorna un errore e il loop termina
                    //il thread così conclude la propria esecuzione e può essere aspettato dal main
                    //nel tratto Drop del synchronizer
                    res = r1.recv();
                    match res {
                        Ok(v) => v1 = v,
                        Err(_) => break,
                    }

                    res = r2.recv();
                    match res {
                        Ok(v) => v2 = v,
                        Err(_) => break,
                    }

                    process(v1, v2);
                }
                println!("Processor is dying...");
            }
        });
        return Arc::new(Synchronizer {
            processor_handle: Some(processor_handle),
            sender1: Some(s1),
            sender2: Some(s2),
        });
    }

    pub fn data_from_first_port(&self, value: K) {
        self.sender1.clone().unwrap().send(value).unwrap();
    }

    pub fn data_from_second_port(&self, value: K) {
        self.sender2.clone().unwrap().send(value).unwrap();
    }
}

//l'implementazione è necessaria, altrimenti il programma non è totalmente corretto
impl<K: Send + 'static + Debug> Drop for Synchronizer<K> {
    fn drop(&mut self) {
        let s1 = self.sender1.take().unwrap();
        drop(s1);
        let s2 = self.sender2.take().unwrap();
        drop(s2);

        let handle = self.processor_handle.take().unwrap();

        handle.join().unwrap();
    }
}

fn printer<K: Debug>(i1: K, i2: K) {
    println!("> Values received = {:?}, {:?}", i1, i2);
}

pub fn test() {
    let synchronizer = Synchronizer::new(printer);

    //il sistema sfrutta due threads per leggere dalle porte
    let h1 = thread::spawn({
        let s = synchronizer.clone();
        move || {
            let port = SerialPort::new();
            for _ in 0..N_READS {
                let val = port.read();
                println!("sending {val} from port 1");
                s.data_from_first_port(val);
            }
        }
    });

    let h2 = thread::spawn({
        let s = synchronizer.clone();
        move || {
            let port = SerialPort::new();
            for _ in 0..N_READS {
                let val = port.read();
                thread::sleep(Duration::from_secs(4));
                s.data_from_second_port(val);
            }
        }
    });

    h1.join().unwrap();
    h2.join().unwrap();
}
