13 gennaio 2025

La struct DelayedExecutor permette di eseguire funzioni in modo asincrono, dopo un certo intervallo di tempo.
Essa offre tre metodi:

    new() -> Self crea un nuovo DelayedExecutor

    execute<F: FnOnce()+Send+'static>(f:F, delay: Duration) -> bool
        se il DelayedExecutor è aperto, accoda la funzione f che dovrà essere eseguita non prima che sia
        trascorso un intervallo pari a delay e restituisce true; se invece il DelayedExecutor è chiuso, restituisce false.

    close(drop_pending_tasks: bool) chiude il DelayedExecutor;
        se drop_pending_tasks è true, le funzioni in attesa di essere eseguite vengono eliminate, altrimenti vengono eseguite a tempo debito.

DelayedExecutor è thread-safe e può essere utilizzato da più thread contemporaneamente.
I task sottomessi al DelayedExecutor devono essere eseguiti in ordine di scadenza.
All'atto della distruzione di un DelayedExecutor, tutti i task in attesa sono eliminati, ma se è in corso
un'esecuzione questa viene portata a termine evitando di creare corse critiche.
Si implementi questa struct in linguaggio Rust.
