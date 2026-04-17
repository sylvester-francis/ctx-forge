//! TTL / freshness evaluation.

use chrono::{DateTime, Utc};
use std::time::Duration;

pub fn is_fresh(fetched_at: DateTime<Utc>, ttl: Duration) -> bool {
    let now = Utc::now();
    let age = now.signed_duration_since(fetched_at);
    match chrono::Duration::from_std(ttl) {
        Ok(d) => age < d,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_is_fresh() {
        assert!(is_fresh(Utc::now(), Duration::from_secs(3600)));
    }

    #[test]
    fn old_is_stale() {
        let t = Utc::now() - chrono::Duration::hours(25);
        assert!(!is_fresh(t, Duration::from_secs(86_400)));
    }
}
