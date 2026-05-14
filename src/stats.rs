use std::time::{Duration, Instant};

pub struct TranslationStats {
    label: String,
    total: usize,
    succeeded: usize,
    failed: usize,
    start: Instant,
    per_item: Vec<Duration>,
}

impl TranslationStats {
    pub fn new(label: impl Into<String>, total: usize) -> Self {
        Self {
            label: label.into(),
            total,
            succeeded: 0,
            failed: 0,
            start: Instant::now(),
            per_item: Vec::new(),
        }
    }

    pub fn record_ok(&mut self, duration: Duration) {
        self.succeeded += 1;
        self.per_item.push(duration);
    }

    pub fn record_ok_batch(&mut self, count: usize) {
        self.succeeded += count;
    }

    pub fn record_fail(&mut self) {
        self.failed += 1;
    }

    pub fn report(&self) -> String {
        let elapsed = self.start.elapsed();
        let avg = if !self.per_item.is_empty() {
            let sum: Duration = self.per_item.iter().sum();
            format!("{:.2}s", sum.as_secs_f64() / self.per_item.len() as f64)
        } else {
            "-".to_string()
        };

        let status = if self.failed == 0 {
            format!("{} ok", self.succeeded)
        } else {
            format!("{} ok, {} failed", self.succeeded, self.failed)
        };

        format!(
            "── {} ──\n  {} string{} in {:.2}s (avg {}, {})",
            self.label,
            self.total,
            if self.total == 1 { "" } else { "s" },
            elapsed.as_secs_f64(),
            avg,
            status,
        )
    }
}
