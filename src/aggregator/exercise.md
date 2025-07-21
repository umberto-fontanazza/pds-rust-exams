Aggregatore di misure

Un sistema di monitoraggio all'interno di uno stabilimento industriale raccoglie misure di temperatura da più
sensori. Le misure vengono raccolte in modo asincrono, sono automaticamente etichettate con
l'istante temporale in cui sono comunicate e possono essere inviate da più thread
contemporaneamente. Compito del sistema è quello di aggregare le misure ricevute, calcolando la
temperatura media e il numero di misurazioni ricevute da ciascun sensore, operando un campionamento ad
intervalli regolari indicati dal parametro passato alla funzione di costruzione. In tale periodo, un sensore può
inviare più misure, che devono essere tutte considerate nel calcolo della media. Un thread interno alla
struttura si occupa di calcolare la media delle temperature per ciascun sensore, aggiornandola secondo il
periodo di campionamento indicato. All'atto della distruzione della struttura, il thread interno deve essere
terminato in modo sicuro. Per implementare tale sistema, si richiede di realizzare la struct Aggregator che
offre i seguenti metodi thread-safe:

use std::time::Instant;

pub struct Aggregator {
    // campi privati
}

pub struct Average {
    pub sensor_id: usize,
    pub reference_time: Instant, //indica l'istante temporale in cui è stata calcolata la media
    pub average_temperature: f64,
}

impl Aggregator {
    pub fn new(sample_time_millis: u64) -> Self {
        // implementazione del costruttore
        todo!()
    }

    pub fn add_measure(&self, sensor_id: usize, temperature: f64) {
        // aggiunge una misura di temperatura per il sensore con id `sensor_id` // e temperatura `temperature`. Le misure sono automaticamente etichettate
        // con l'istante temporale in cui sono comunicate.
        todo!()
    }

    pub fn get_averages(&self) -> Vec<Average> {
        // restituisce un vettore che riporta la temperatura media di ciascun sensore,
        // calcolata durante l'ultimo periodo di campionamento.
        // Sono presenti solo i sensori che hanno inviato almeno una misura.
        todo!()
    }
}