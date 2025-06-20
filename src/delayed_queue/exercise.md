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
    tempo, offerto dai metodi wait_timeout(...) e wait_timeout_while(...).