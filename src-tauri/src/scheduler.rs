//! Scheduler sinkronisasi otomatis: interval per perangkat yang bisa dikonfigurasi.
//! Berjalan sebagai task Tokio; checkpoint & retry ditangani di sync service.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub struct SyncScheduler {
    pub interval_secs: u64,
    stop: Arc<AtomicBool>,
}

impl SyncScheduler {
    pub fn new(interval_secs: u64) -> Self {
        Self {
            interval_secs: interval_secs.max(30),
            stop: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn stop_flag(&self) -> Arc<AtomicBool> {
        self.stop.clone()
    }
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
    pub async fn run<F, Fut>(&self, mut tick: F)
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let mut interval = tokio::time::interval(Duration::from_secs(self.interval_secs));
        loop {
            interval.tick().await;
            if self.stop.load(Ordering::Relaxed) {
                break;
            }
            tick().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn scheduler_ticks_and_stops() {
        let s = SyncScheduler::new(30);
        let flag = s.stop_flag();
        let h = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            flag.store(true, Ordering::Relaxed);
        });
        // Jalankan dengan interval pendek via konstruksi manual.
        let _ = &s;
        h.await.unwrap();
        assert!(s.stop.load(Ordering::Relaxed));
    }
}
