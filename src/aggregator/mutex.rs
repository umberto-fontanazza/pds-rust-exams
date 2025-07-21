use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

struct Measurement {
    id: usize,
    timestamp: Instant,
    measure: f64,
}

struct InnerState {
    running: bool,
    measurements: Vec<Measurement>,
    sample_time: Instant,
    recent_averages: Vec<Average>,
}

#[derive(PartialEq, Clone, Debug)]
pub struct Average {
    pub sensor_id: usize,
    pub reference_time: Instant, //indica l'istante temporale in cui è stata calcolata la media
    pub average_temperature: f64,
}

pub struct Aggregator {
    // campi privati
    state: Arc<(Mutex<InnerState>, Condvar)>,
    join_handle: Option<JoinHandle<()>>,
}

impl Aggregator {
    pub fn new(sample_time_millis: u64) -> Self {
        // implementazione del costruttore
        let inner_state = Arc::new((
            Mutex::new(InnerState {
                running: true,
                measurements: vec![],
                sample_time: Instant::now(),
                recent_averages: vec![],
            }),
            Condvar::new(),
        ));
        let state = inner_state.clone();
        let join_handle = std::thread::spawn(move || {
            let mut inner_state = state.0.lock().unwrap();
            loop {
                let next_wakeup =
                    inner_state.sample_time + Duration::from_millis(sample_time_millis);
                let sleep_time = next_wakeup.saturating_duration_since(Instant::now());
                inner_state = state
                    .1
                    .wait_timeout_while(inner_state, sleep_time, |s| s.running)
                    .unwrap()
                    .0;

                if !inner_state.running {
                    break;
                }
                inner_state.sample_time = next_wakeup;
                let mut measurement = inner_state
                    .measurements
                    .extract_if(.., |m| m.timestamp < next_wakeup)
                    .collect::<Vec<_>>();
                drop(inner_state);
                let mut averages = HashMap::<usize, (f64, usize)>::new();
                measurement.iter_mut().for_each(|m| {
                    averages
                        .entry(m.id)
                        .and_modify(|(sum, count)| {
                            *sum += m.measure;
                            *count += 1;
                        })
                        .or_insert((m.measure, 1));
                });
                let new_averages: Vec<Average> = averages
                    .iter()
                    .map(|(id, (measure, count))| Average {
                        sensor_id: *id,
                        reference_time: next_wakeup,
                        average_temperature: *measure / *count as f64,
                    })
                    .collect();
                inner_state = state.0.lock().unwrap();
                inner_state.recent_averages = new_averages;
            }
        });
        Self {
            state: inner_state,
            join_handle: Some(join_handle),
        }
    }

    pub fn add_measure(&self, sensor_id: usize, temperature: f64) {
        let now = Instant::now();
        let mut state = self.state.0.lock().unwrap();
        state.measurements.push(Measurement {
            id: sensor_id,
            timestamp: now,
            measure: temperature,
        });
    }

    pub fn get_averages(&self) -> Vec<Average> {
        // restituisce un vettore che riporta la temperatura media calcolata durante l'ultimo periodo di campionamento
        // da ciascun sensore. Sono presenti solo i sensori che hanno inviato almeno una misura.
        let state = self.state.0.lock().unwrap();
        state.recent_averages.clone()
    }
}

impl Drop for Aggregator {
    fn drop(&mut self) {
        let mut state = self.state.0.lock().unwrap();
        state.running = false;
        drop(state);
        self.state.1.notify_all();
        let join_handle = self.join_handle.take().unwrap();
        join_handle.join().unwrap();
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
