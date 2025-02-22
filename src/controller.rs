use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{interval, Duration, Instant};

use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug)]
pub struct TrafficController {
    sem: Arc<Semaphore>,
    capacity: usize,
}

impl TrafficController {
    pub async fn new(capacity: usize) -> Self {
        let sem = Arc::new(Semaphore::new(capacity));
        let permit = sem
            .acquire_many(capacity.try_into().unwrap())
            .await
            .unwrap();
        permit.forget();

        Self { sem, capacity }
    }

    pub async fn flow(&self, ramp_up_sec: u64, cancel_token: CancellationToken) {
        let mut update_interval = interval(Duration::from_secs_f64(1.0 / self.capacity as f64));
        let inital_per_sec = 0.5;
        if ramp_up_sec > 0 {
            update_interval = interval(Duration::from_secs_f64(inital_per_sec));
            update_interval.tick().await;
        }
        update_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let ramp_up_start = Instant::now();

        let local_token = cancel_token.clone();
        loop {
            tokio::select! {
                _ = local_token.cancelled() => {
                    break;
                },
                _ = update_interval.tick() => {
                    if self.sem.available_permits() < self.capacity {
                        self.sem.add_permits(1);
                    }

                    if ramp_up_sec > 0 {
                        let elapsed_seconds = ramp_up_start.elapsed().as_secs_f64();
                        if elapsed_seconds <= ramp_up_sec as f64 {
                            let next_per_sec = 1.0 / ( self.capacity as f64 * elapsed_seconds/ramp_up_sec as f64);
                            if next_per_sec < inital_per_sec {
                                update_interval = interval(Duration::from_secs_f64(
                                    next_per_sec
                                ));
                                update_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                                update_interval.tick().await;
                            }
                        }
                    }
                }
            }
        }
    }

    pub async fn acquire(&self) {
        let permit = self.sem.acquire().await.unwrap();
        permit.forget();
    }
}
