use std::time::{SystemTime, UNIX_EPOCH};

/// Source of timestamps (milliseconds since the Unix epoch). Replaced in tests.
pub trait Clock: Send + Sync {
  fn now_ms(&self) -> i64;
}

pub struct SystemClock;

impl Clock for SystemClock {
  fn now_ms(&self) -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
  }
}
