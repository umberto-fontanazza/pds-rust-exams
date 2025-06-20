2022-10-26
Un componente con funzionalità di cache permette di ottimizzare il comportamento di un sistema riducendo il numero di volte in cui una funzione è invocata,
tenendo traccia dei risultati da essa restituiti a fronte di un particolare dato in ingresso. Per generalità, si assuma che la funzione accetti un dato di
tipo generico K e restituisca un valore di tipo generico V.

Il componente offre un unico metodo get(...) che prende in ingresso due parametri, il valore k (di tipo K, clonabile) del parametro e la funzione f (di tipo K -> V)
responsabile della sua trasformazione, e restituisce uno smart pointer clonabile al relativo valore.

Se, per una determinata chiave k, non è ancora stato calcolato il valore corrispondente, la funzione viene invocata e ne viene restituito il risultato;
altrimenti viene restituito il risultato già trovato.

Il componente cache deve essere thread-safe perché due o più thread possono richiedere contemporaneamente il valore di una data chiave: quando questo avviene e il dato
non è ancora presente, la chiamata alla funzione dovrà essere eseguita nel contesto di UN SOLO thread, mentre gli altri dovranno aspettare il risultato in corso di
elaborazione, SENZA CONSUMARE cicli macchina.

Si implementi tale componente a scelta nei linguaggi C++ o Rust.
