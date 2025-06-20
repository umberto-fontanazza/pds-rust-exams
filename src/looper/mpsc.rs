/*
2022-06-20
Un paradigma frequentemente usato nei sistemi reattivi e costituito dall'astrazione detta Looper.
Quando viene creato, un Looper crea una coda di oggetti generici di tipo Message ed un thread.
II thread attende - senza consumare cicli di CPU - che siano presenti messaggi nella coda,
li estrae a uno a uno nell'ordine di arrivo, e li elabora.

II costruttore di Looper riceve due parametri, entrambi di tipo (puntatore a) funzione: process( ... ) e cleanup().
La prima è una funzione responsabile di elaborare i singoli messaggi ricevuti attraverso la coda;
tale funzione accetta un unico parametro in ingresso di tipo Message e non ritorna nulla;
La seconda e funzione priva di argomenti e valore di ritorno e verra invocata dal thread incapsulato
nel Looper quando esso stara per terminare.

Looper offre un unico metodo pubblico, thread safe, oltre a quelli di servizio, necessari per gestirne ii ciclo di vita:
send(msg), che accetta come parametro un oggetto generico di tipo Message che verra inserito nella coda
e successivamente estratto dal thread ed inoltrato alla funzione di elaborazione.
Quando un oggetto Looper viene distrutto, occorre fare in modo che ii thread contenuto al suo interno
invochi la seconda funzione passata nel costruttore e poi termini.

Si implementi, utilizzando il linguaggio Rust o C++, tale astrazione tenendo conto che i suoi
 metodi dovranno essere thread-safe.
*/

use std::sync::Arc;
use std::sync::mpsc::{Sender, channel};
use std::thread::{JoinHandle, spawn};

pub struct Looper<T: Send> {
    sender: Option<Sender<T>>,
    thread: Option<JoinHandle<()>>,
}

impl<T: Send + 'static> Looper<T> {
    pub fn new<P, C>(mut process: P, cleanup: C) -> Self
    where
        P: FnMut(T) -> () + Send + 'static,
        C: FnOnce() -> () + Send + 'static,
    {
        let (sender, receiver) = channel::<T>();
        let thread = spawn(move || {
            while let Ok(message) = receiver.recv() {
                process(message)
            }
            cleanup();
        });
        let sender = Some(sender);
        let thread = Some(thread);
        Self { sender, thread }
    }

    pub fn send(&self, message: T) {
        self.sender.as_ref().unwrap().send(message).unwrap();
    }
}

impl<T: Send> Drop for Looper<T> {
    fn drop(&mut self) {
        self.sender.take().unwrap();
        self.thread.take().unwrap().join().unwrap();
    }
}

pub fn test() {
    let process = |number: usize| {
        println!("Thread {number} sent to the Looper!");
    };
    let cleanup = || {
        println!("Cleaning up");
    };
    let my_looper = Arc::new(Looper::new(process, cleanup));

    let (count_threads, count_messages) = (5, 5);
    let mut threads = Vec::<JoinHandle<()>>::with_capacity(count_threads);
    for thread_id in 0..count_threads {
        let looper_arc = my_looper.clone();
        threads.push(spawn(move || {
            (0..count_messages).for_each(|_| {
                looper_arc.send(thread_id);
            });
        }));
    }
    threads
        .into_iter()
        .for_each(|handle| handle.join().unwrap());
}
