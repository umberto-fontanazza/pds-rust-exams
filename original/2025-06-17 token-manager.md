# Programmazione di Sistema - Esame del 17 Giugno 2025

## Esercizio 1: Analisi dell'allocazione di memoria (Stack e Heap)

### A

Si considerino le seguenti strutture dati e rispettive porzioni di codice. Per ciascuna di esse si indichi la dimensione di memoria allocata nello stack e nello heap, ipotizzando un'architettura a 64 bit.

```Rust
let boxed: Box<[u32]> = vec![10, 20, 30, 40, 50].into_boxed_slice();
let slice_ref: &[u32] = &boxed[1..4];
```

Domande:

1. Qual è la dimensione della variabile boxed nello stack? (0.4 pt)
2. Quanta memoria è allocata nello heap dopo l'esecuzione del codice? (0.4 pt)
3. Qual è lo spazio di memoria occupato da slice_ref nello stack? (0.4 pt)
4. La slice slice_ref punta alla stessa memoria heap della boxed? (0.3 pt)

### B

Si considerino le seguenti strutture dati e porzioni di codice.

Per ciascuna di esse si indichi la dimensione di memoria allocata nello stack e nello heap, ipotizzando un’architettura a 64 bit.

```Rust
use std::cell::RefCell;

let shared_vec: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(vec![1, 2, 3]));

let shared_clone = Rc::clone(&shared_vec);

shared_clone.borrow_mut().push(4);
```

Domande:

1. Quanta memoria è usata nello stack da shared_vec e shared_clone? (0.5 pt)
2. Quanto spazio è allocato nello heap in totale (considerando Rc, RefCell, e Vec)? (0.5 pt)
3. Dopo l’esecuzione del codice, quanti riferimenti forti e deboli esistono? (0,3 pt)
4. È possibile accedere contemporaneamente in lettura da shared_vec e in scrittura da shared_clone? (0,2 pt)

---

## Esercizio 2: Monomorfizzazione in Rust

- Si spieghi che cosa si intende per monomorfizzazione ? (0.5 pt)
- Si mostri una porzione di codice che mostri un esempio pratico in cui la monomorfizzazione viene applicata (0.5 pt)
- Si mostri i principali punti di forza e i punti di debolezza di tale proprietà (0.5 pt)

---

## Esercizio 3: Concorrenza con Produttore-Consumatore

Si consideri il seguente programma Rust che usa due thread per gestire una coda condivisa. Le righe sono numerate.

```Rust
1 use std::sync::{Arc, Mutex, Condvar};
2 use std::thread;
3 use std::collections::VecDeque;
4
5 fn main() {
6     let queue = Arc::new((Mutex::new(VecDeque::<u32>::new()), Condvar::new()));
7
8     let queue_producer = Arc::clone(&queue);
9     let queue_cons = Arc::clone(&queue);
10
11     let producer = thread::spawn(move || {
12         let (lock, cvar) = &*queue_producer;
13         for i in 1..=3 {
14             thread::sleep(std::time::Duration::from_millis(500));
15             let mut queue = lock.lock().unwrap();
16             queue.push_back(i);
17             cvar.notify_one();
18         }
19     });
20
21     let consumer = thread::spawn(move || {
22         let (lock, cvar) = &*queue_cons;
23         loop {
24             let mut queue = lock.lock().unwrap();
25             while queue.is_empty() {
26                 queue = cvar.wait(queue).unwrap();
27             }
28             if let Some(val) = queue.pop_front() {
29                 println!("Consumer 1: got {}", val);
30             } else {
31                 println!("(A)");
32             }
33         }
34     });
35
36     producer.join().unwrap();
37     consumer.join().unwrap();
38 }
```

Domande:

1. Descrivere il comportamento complessivo del programma e spiegare cosa fanno i 2 thread. (0.2 pt)
2. È possibile che venga stampato (A) alla riga 31? Se sì, in quali condizioni? (0.2 pt)
3. Il programma termina? Se no, spiegare perché e proporre una modifica per permettere una
   terminazione pulita dei thread consumatori. (0.8 pt)
4. Ci sono potenziali race condition o deadlock? Giustificare. (0.3 pt)

---

## Esercizio 4: Programmazione - Token Manager

Un applicativo software multithread fa accesso ai servizi di un server remoto, attraverso richieste di tipo HTTP.
Tali richieste devono includere un token di sicurezza che identifica l'applicativo stesso e ne autorizza l'accesso.
Per motivi di sicurezza, il token ha una validità limitata nel tempo (qualche minuto) e deve essere rinnovato alla sua scadenza.
Il token viene ottenuto attraverso una funzione (fornita esternamente e conforme al tipo **TokenAcquirer**) che restituisce alternativamente un token e la sua data di scadenza o un messaggio di errore se non è possibile fornirlo.
Poiché la emissione richiede un tempo apprezzabile (da alcune centinaia di millisecondi ad alcuni secondi), si vuole centralizzare la gestione del token,
per evitare che più thread ne facciano richiesta in contemporanea.

A tale scopo deve essere implementata la struct TokenManager che si occupa di gestire il rilascio, il rinnovo e la messa a disposizione del token a chi ne abbia bisogno, secondo la logica di seguito indicata.

La struct **TokenManager** offre i seguenti metodi:

```Rust
type TokenAcquirer = dyn Fn() -> Result<(String, Instant), String> + Sync;

pub fn new(acquire_token: Box<TokenAcquirer> ) -> Self
pub fn get_token(&self) -> Result<String, String>
pub fn try_get_token(&self) -> Option<string>
```

Al proprio interno, la struct TokenManager mantiene 3 possibili stati:

- Empty - indica che non è ancora stato richiesto alcun token;
- Pending - indica che è in corso una richiesta di acquisizione del token;
- Valid - indica che è disponibile un token in corso di validità;

Il metodo `new(...)` riceve il puntatore alla funzione in grado di acquisire il token. Essa opera in modalità pigra e si limita a creare un'istanza della struttura con le necessarie informazioni per gestire il suo successivo comportamento.

Il metodo `get_token(...)` implementa il seguente comportamento:

- Se lo stato è Empty, passa allo stato Pending e invoca la funzione per acquisire il token; se questa ritorna un risultato valido, memorizza il token e la sua scadenza, porta lo stato a Valid e restituisce copia del token stesso; <br>se, invece, questa restituisce un errore, pone lo stato a Empty e restituisce l'errore ricevuto.
- Se lo stato è Pending, attende senza consumare cicli di CPU che questo passi ad un altro valore, dopodiché si comporta di conseguenza.
- Se lo stato è Valid e il token non risulta ancora scaduto, ne restituisce una copia; altrimenti pone lo stato ad Pending e inizia una richiesta di acquisizione, come indicato sopra.

Il metodo `try_get_token(...)` implementa il seguente comportamento:

- Se lo stato è Valid e il token non è scaduto, restituisce una copia del token opportunamente incapsulata in un oggetto di tipo Option. In tutti gli altri casi restituisce None.

Si implementi tale struttura nel linguaggio Rust.

A supporto della validazione del codice realizzato si considerino i seguenti test (due dei quali sono forniti con la relativa implementazione, i restanti sono solo indicati e possono essere opportunamente completati):

```Rust
    #[test]
    fn a_new_manager_contains_no_token() {
        let a: Box<TokenAcquirer> = Box::new(|| Err("failure".to_string()));
        let manager = TokenManager::new(a);
        assert!(manager.try_get_token().is_none());
    }
    #[test]
    fn a_failing_acquirer_always_returns_an_error() {
        let a: Box<TokenAcquirer> = Box::new(|| Err("failure".to_string()));
        let manager = TokenManager::new(a);
        assert_eq!(manager.get_token(), Err("failure".to_string()));
        assert_eq!(manager.get_token(), Err("failure".to_string()));
    }
    #[test]
    fn a_successful_acquirer_always_returns_success() {
      //...to be implemented
    }
    #[test]
    fn a_slow_acquirer_causes_other_threads_to_wait() {
      //...to be implemented
    }
```
