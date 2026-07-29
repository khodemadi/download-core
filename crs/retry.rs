use std::time::Duration;
use tokio::time::sleep;

pub async fn backoff(attempt: usize) {
    let capped = attempt.min(6) as u32;
    let ms = 250u64.saturating_mul(2u64.saturating_pow(capped));
    sleep(Duration::from_millis(ms.min(10_000))).await; } 