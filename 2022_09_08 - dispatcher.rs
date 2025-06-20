/*
In un sistema concorrente, ciascun thread può pubblicare eventi per rendere noto ad altri thread quanto sta facendo.
Per evitare un accoppiamento stretto tra mittenti e destinatari degli eventi, si utilizza un Dispatcher: questo è un oggetto thread-safe che offre il metodo

        dispatch(msg: Msg)

mediante il quale un messaggio di tipo generico Msg (soggetto al vincolo di essere clonabile) viene reso disponibile a chiunque si sia sottoscritto.
Un thread interessato a ricevere messaggi può invocare il metodo

        subscribe()

del Dispatcher: otterrà come risultato un oggetto di tipo Subscription mediante il quale potrà leggere i messaggi che da ora in poi saranno pubblicati attraverso
il Dispatcher. Per ogni sottoscrizione attiva, il Dispatcher mantiene internamente l'equivalente di una coda ordinata (FIFO) di messaggi non ancora letti.
A fronte dell'invocazione del metodo dispatch(msg:Msg), il messaggio viene clonato ed inserito in ciascuna delle code esistenti. L'oggetto Subscription offre il
metodo bloccante

        read() -> Option<Msg>

se nella coda corrispondente è presente almeno un messaggio, questo viene rimosso e restituito; se nella coda non è presente nessun messaggio e il Dispatcher esiste
ancora, l'invocazione si blocca fino a che non viene inserito un nuovo messaggio; se invece il Dispatcher è stato distrutto, viene restituito il valore corrispondente
all'opzione vuota.

Gli oggetti Dispatcher e Subscription sono in qualche modo collegati, ma devono poter avere cicli di vita indipendenti: la distruzione del Dispatcher non deve impedire la
consumazione dei messaggi già recapitati ad una Subscription, ma non ancora letti; parimenti, la distruzione di una Subscription non deve impedire al Dispatcher di
consegnare ulteriori messaggi alle eventuali altre Subscription presenti.

Si implementino le strutture dati Dispatcher e Subscription, a scelta, nel linguaggio Rust o C++11.
*/

use rand::Rng;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::sleep;
use std::time::Duration;

struct Dispatcher<Msg: Clone> {
    senders_vec: Mutex<Vec<Sender<Msg>>>,
}

impl<Msg: Clone> Dispatcher<Msg> {
    fn new() -> Dispatcher<Msg> {
        Dispatcher {
            senders_vec: Mutex::new(vec![]),
        }
    }

    fn subscribe(&self) -> Subscription<Msg> {
        let (tx, rx) = channel();
        let mut lock = self.senders_vec.lock().unwrap();
        (*lock).push(tx);
        Subscription::new(rx)
    }

    fn dispatch(&self, msg: Msg) {
        let lock = self.senders_vec.lock().unwrap();

        for sender in lock.iter() {
            let _ = sender.send(msg.clone()); //ritorna errore se la subscription del receiver associato è stata droppata, quindi non fare unwrap altrimenti panica
        }
    }
}

//aggiunto per ragioni di debug
//la distruione del dispatcher implica la distruzione dei sender, che interrompe l'attesa sulle recv()
impl<Msg: Clone> Drop for Dispatcher<Msg> {
    fn drop(&mut self) {
        println!("\n!!!! > Dispatcher is dropped HERE! < !!!!\n");
    }
}

struct Subscription<Msg> {
    sub: Receiver<Msg>,
}

impl<Msg: Clone> Subscription<Msg> {
    fn new(rx: Receiver<Msg>) -> Self {
        Subscription { sub: rx }
    }

    fn read(&self) -> Option<Msg> {
        let msg = self.sub.recv();
        if msg.is_ok() {
            Some(msg.unwrap())
        } else {
            None
        }
    }
}

//aggiunto per ragioni di debug per mostrare che le subscription sono indipendenti l'una dall'altra
impl<Msg> Drop for Subscription<Msg> {
    fn drop(&mut self) {
        println!(" '-> It's subscription is dropped!\n");
    }
}

fn main() {
    let dispatcher = Arc::new(Dispatcher::new());

    let mut handles = vec![];

    for i in 0..10 {
        handles.push(thread::spawn({
            //clono il riferimento al dispatcher in modo da poterlo chiamare da più threads
            let d = dispatcher.clone();
            move || {
                let time = rand::thread_rng().gen_range(0..5);
                sleep(Duration::from_secs(time));
                let sub = d.subscribe();
                //il dispatcher è multiple-producer e può essere utilizzato da più threads insieme
                d.dispatch("from thread ".to_string() + i.to_string().as_str());
                //è ESSENZIALE effettuare la drop del riferimento prima di fare la read()
                // infatti dispatcher rimane in vita finché esiste almeno un riferimento ad esso
                // e se il thread possiede un riferimento mentre fa la read richia di mandarsi da solo in deadlock
                std::mem::drop(d);
                loop {
                    let time = rand::thread_rng().gen_range(10..100);
                    sleep(Duration::from_millis(time)); //helps print to remain mostly in-order
                    let res = sub.read();
                    match res {
                        None => {
                            println!("Thread {i} returns DUE TO THE DISPATCHER");
                            break;
                        }
                        Some(msg) => {
                            println!("    thread {} received msg {} ", i, msg)
                        }
                    }
                    let early_drop = rand::thread_rng().gen_range(0..10);
                    if early_drop == 0 {
                        println!("Thread {i} returns EARLY");
                        drop(sub);
                        break;
                    }
                }
            }
        }))
    }

    for i in 30..35 {
        println!("> Dispatching value {i}");
        let time = rand::thread_rng().gen_range(2..4);
        sleep(Duration::from_secs(time));
        dispatcher.dispatch(i.to_string() + " from main");
    }

    //il main possiede un riferimento al dispatcher che deve essere eliminato (insieme a tutti gli altri)
    // per poter permettere alle read() in attesa di ritornare, una volta che non si vogliono più inviare messaggi
    drop(dispatcher);

    for h in handles {
        h.join().unwrap();
    }
}
