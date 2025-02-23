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

    pub async fn flow(&self, ramp_up_sec: f64, cancel_token: CancellationToken) {
        let mut update_interval = interval(Duration::from_secs_f64(1.0 / self.capacity as f64));
        let inital_per_sec = 0.8;
        if ramp_up_sec > 0.0 {
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

                    if ramp_up_sec > 0.0 {
                        let elapsed_seconds = ramp_up_start.elapsed().as_secs_f64();
                        if elapsed_seconds <= ramp_up_sec {
                            let next_per_sec = 1.0 / ( self.capacity as f64 * elapsed_seconds/ramp_up_sec);
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

    pub async fn acquire(&self) {
        let permit = self.sem.acquire().await.unwrap();
        permit.forget();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_traffic_controller_new() {
        let capacity = 5;
        let controller = TrafficController::new(capacity).await;
        assert_eq!(controller.capacity, capacity);
        assert_eq!(controller.sem.available_permits(), 0);
    }

    #[tokio::test]
    async fn test_traffic_controller_flow_no_ramp_up() {
        let controller = Arc::new(TrafficController::new(4).await);
        let cancel_token = CancellationToken::new();

        let local_controller = controller.clone();
        let local_cancel_token = cancel_token.clone();
        tokio::spawn(async move {
            local_controller.flow(0.0_f64, local_cancel_token).await;
        });

        sleep(Duration::from_millis(100)).await;

        assert!(controller.sem.available_permits() > 0);
        cancel_token.cancel();
    }

    #[tokio::test]
    async fn test_traffic_controller_flow_with_ramp_up() {
        let capacity = 10;
        let ramp_up_sec = 4.0;
        let controller = Arc::new(TrafficController::new(capacity).await);
        let cancel_token = CancellationToken::new();

        let local_controller = controller.clone();
        let local_cancel_token = cancel_token.clone();
        tokio::spawn(async move {
            local_controller.flow(ramp_up_sec, local_cancel_token).await;
        });

        let mut now = Instant::now();
        for _ in 0..10 {
            assert!(controller.sem.available_permits() <= capacity);
            controller.acquire().await;
            let new_now = Instant::now();
            assert!(
                new_now.saturating_duration_since(now).as_millis()
                    >= (1.0 / capacity as f32 * 1000.0) as u128
            );
            now = new_now;
        }

        cancel_token.cancel();
    }
}
