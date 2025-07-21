use std::collections::HashMap;
use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::thread::{JoinHandle, spawn};
use std::time::{Duration, Instant};

type SensorId = usize;
type Measurement = (SensorId, f64, Instant);

enum DaemonMessage {
    AddMeasure(Measurement),
    AveragesRequest(Sender<Vec<Average>>),
}

pub struct Aggregator {
    // campi privati
    sender: Option<Sender<DaemonMessage>>,
    daemon: Option<JoinHandle<()>>,
}

#[derive(PartialEq, Clone)]
pub struct Average {
    pub sensor_id: usize,
    pub reference_time: Instant, //indica l'istante temporale in cui è stata calcolata la media
    pub average_temperature: f64,
}

impl Aggregator {
    pub fn new(sample_time_millis: u64) -> Self {
        let aggregation_period = Duration::from_millis(sample_time_millis);
        let (snd, rx) = channel::<DaemonMessage>();
        let daemon = {
            Some(spawn(move || {
                let mut start_t = Instant::now();
                let mut end_t = start_t + aggregation_period;
                let mut measures = HashMap::<SensorId, Vec<f64>>::new();
                let mut averages: Option<Vec<Average>> = None;
                loop {
                    match rx.recv_timeout(end_t - Instant::now()) {
                        Ok(DaemonMessage::AddMeasure((sensor_id, temperature, measure_time))) => {
                            // TODO: ask Malnati for this one
                            // if measure_time < start_t {
                            //     println!("Old measure received!");
                            //     continue;
                            // }
                            // if measure_time >= end_t {
                            //     println!("Received a measure in the future!");
                            //     continue;
                            // }
                            measures
                                .entry(sensor_id)
                                .and_modify(|v| v.push(temperature))
                                .or_insert(vec![temperature]);
                        }
                        Ok(DaemonMessage::AveragesRequest(average_snd)) => {
                            average_snd
                                .send(
                                    averages
                                        .as_ref()
                                        .map(|v| v.clone())
                                        .or(Some(Vec::new()))
                                        .unwrap(),
                                )
                                .unwrap();
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            start_t = end_t;
                            end_t = start_t + aggregation_period;
                            averages = Some(
                                std::mem::take(&mut measures)
                                    .into_iter()
                                    .map(|(s_id, v)| {
                                        let mut sum = 0.0;
                                        v.iter().for_each(|value| {
                                            sum += value;
                                        });
                                        Average {
                                            sensor_id: s_id,
                                            reference_time: start_t,
                                            average_temperature: sum / (v.len() as f64),
                                        }
                                    })
                                    .collect(),
                            );
                        }
                        _ => {
                            break;
                        }
                    }
                }
            }))
        };
        Self {
            sender: Some(snd),
            daemon,
        }
    }

    pub fn add_measure(&self, sensor_id: usize, temperature: f64) {
        // aggiunge una misura di temperatura per il sensore con id `sensor_id` // e temperatura `temperature`. Le misure sono automaticamente etichettate
        // con l'istante temporale in cui sono comunicate.
        let now = Instant::now();
        self.sender
            .as_ref()
            .unwrap()
            .send(DaemonMessage::AddMeasure((sensor_id, temperature, now)))
            .unwrap();
    }

    pub fn get_averages(&self) -> Vec<Average> {
        // restituisce un vettore che riporta la temperatura media di ciascun sensore,
        // calcolata durante l'ultimo periodo di campionamento.
        // Sono presenti solo i sensori che hanno inviato almeno una misura.
        let (snd, rx) = channel::<Vec<Average>>();
        self.sender
            .as_ref()
            .unwrap()
            .send(DaemonMessage::AveragesRequest(snd))
            .unwrap();
        rx.recv().unwrap()
    }
}

impl Drop for Aggregator {
    fn drop(&mut self) {
        self.sender.take().unwrap();
        self.daemon.take().unwrap().join().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn when_no_measures_are_sent_an_empty_state_is_returned() {
        let aggregator = Aggregator::new(10);
        let averages = aggregator.get_averages();
        assert!(averages.is_empty());
    }

    #[test]
    fn when_a_single_measure_is_sent_it_is_returned() {
        let aggregator = Aggregator::new(20);
        std::thread::sleep(std::time::Duration::from_millis(1));
        aggregator.add_measure(1, 1.0);
        assert!(aggregator.get_averages().is_empty());
        std::thread::sleep(Duration::from_millis(25));
        let averages = aggregator.get_averages();
        assert_eq!(averages.len(), 1);
        assert!(matches!(
            averages.get(0),
            Some(&Average {
                sensor_id: 1,
                average_temperature: 1.0,
                ..
            })
        ));
    }
    #[test]
    fn when_two_measures_are_sent_their_average_is_returned() {
        let aggregator = Aggregator::new(100);
        aggregator.add_measure(1, 1.0);
        aggregator.add_measure(1, 2.0);
        std::thread::sleep(Duration::from_millis(110));
        let averages = aggregator.get_averages();
        assert_eq!(averages.len(), 1);
        assert!(matches!(
            averages.get(0),
            Some(&Average {
                sensor_id: 1,
                average_temperature: 1.5,
                ..
            })
        ));
    }
    #[test]
    fn when_two_measures_are_sent_from_different_sensors_their_average_is_returned() {
        let aggregator = Aggregator::new(100);
        aggregator.add_measure(1, 1.0);
        aggregator.add_measure(2, 2.0);
        aggregator.add_measure(2, 1.0);
        aggregator.add_measure(1, 2.0);
        std::thread::sleep(Duration::from_millis(110));
        let averages = aggregator.get_averages();
        assert_eq!(averages.len(), 2);
        let timestamp = averages.get(0).unwrap().reference_time;
        assert!(averages.contains(&Average {
            sensor_id: 1,
            average_temperature: 1.5,
            reference_time: timestamp
        }));
        assert!(averages.contains(&Average {
            sensor_id: 2,
            average_temperature: 1.5,
            reference_time: timestamp
        }));
    }

    #[test]
    fn more_threads_may_send_data() {
        let aggregator = Aggregator::new(100);
        std::thread::scope(|s| {
            s.spawn(|| {
                aggregator.add_measure(1, 1.0);
                std::thread::sleep(Duration::from_millis(5));
                aggregator.add_measure(1, 3.0);
            });
            s.spawn(|| {
                aggregator.add_measure(2, 2.0);
                std::thread::sleep(Duration::from_millis(5));
                aggregator.add_measure(2, 8.0);
            });
        });
        std::thread::sleep(Duration::from_millis(110));
        let averages = aggregator.get_averages();
        assert_eq!(averages.len(), 2);
        let timestamp = averages.get(0).unwrap().reference_time;
        assert!(averages.contains(&Average {
            sensor_id: 1,
            average_temperature: 2.0,
            reference_time: timestamp
        }));
        assert!(averages.contains(&Average {
            sensor_id: 2,
            average_temperature: 5.0,
            reference_time: timestamp
        }));
    }
    #[test]
    fn an_aggregator_shuts_down_cleanly() {
        {
            let _aggregator = Aggregator::new(10);
        }
        assert!(true);
    }
}
