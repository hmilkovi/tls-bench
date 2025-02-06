use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{interval, Duration};

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

    pub async fn flow(&self, cancel_token: CancellationToken) {
        let mut update_interval = interval(Duration::from_secs_f64(1.0 / self.capacity as f64));
        update_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
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
                }
            }
        }
    }

    pub async fn acquire(&self) {
        let permit = self.sem.acquire().await.unwrap();
        permit.forget();
    }
}
