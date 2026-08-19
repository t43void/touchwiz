//! Per-key and per-bigram error/latency aggregation for heatmaps and analytics.
//!
//! A [`Heatmap`] accumulates hit counts, error counts, and latency totals so the
//! results screen can color each key and surface the slowest keys and most
//! error-prone bigrams (specification Sections 5 and 10).

use std::collections::HashMap;

use crate::session::Session;

/// Health classification for a single key, driving heatmap color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyHealth {
    /// Low error rate and on-target latency.
    Good,
    /// Marginal error rate or slightly slow.
    Warning,
    /// High error rate or significantly slow.
    Bad,
}

/// Rolling statistics for one trainable unit (a key or a bigram).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UnitStat {
    /// Total attempts.
    pub hits: u32,
    /// Incorrect attempts.
    pub errors: u32,
    /// Sum of measured latencies in milliseconds (for averaging).
    pub total_latency_ms: f64,
}

impl UnitStat {
    /// Error rate in `[0, 1]`; `0.0` when never hit.
    pub fn error_rate(&self) -> f64 {
        if self.hits == 0 {
            0.0
        } else {
            f64::from(self.errors) / f64::from(self.hits)
        }
    }

    /// Average latency in milliseconds; `0.0` when never hit.
    pub fn avg_latency_ms(&self) -> f64 {
        if self.hits == 0 {
            0.0
        } else {
            self.total_latency_ms / f64::from(self.hits)
        }
    }

    /// Classifies health against a latency threshold (Section 5 color rules).
    pub fn health(&self, latency_threshold_ms: f64) -> KeyHealth {
        let err = self.error_rate();
        let slow = self.avg_latency_ms();
        if err > 0.05 || slow > latency_threshold_ms * 1.5 {
            KeyHealth::Bad
        } else if err > 0.01 || slow > latency_threshold_ms {
            KeyHealth::Warning
        } else {
            KeyHealth::Good
        }
    }
}

/// Aggregated per-key and per-bigram statistics.
#[derive(Debug, Clone, Default)]
pub struct Heatmap {
    keys: HashMap<char, UnitStat>,
    bigrams: HashMap<String, UnitStat>,
}

impl Heatmap {
    /// Creates an empty heatmap.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds a heatmap from a session's keystrokes.
    ///
    /// Per-key latency uses the press-to-press inter-key interval (the proxy
    /// available without key-release events); bigram latency is attributed to
    /// the second key of each pair.
    pub fn from_session(session: &Session) -> Self {
        let mut hm = Heatmap::new();
        let mut prev_press: Option<u64> = None;
        let mut prev_char: Option<char> = None;

        // Interleave advances and misses by press time so latency stays sane.
        let mut events: Vec<&crate::session::Keystroke> = session
            .keystrokes()
            .iter()
            .chain(session.misses().iter())
            .collect();
        events.sort_by_key(|k| k.press_ms);

        for k in events {
            let latency = match prev_press {
                Some(p) => k.press_ms.saturating_sub(p) as f64,
                None => 0.0,
            };
            if let Some(c) = k.expected.chars().next() {
                hm.record_key(c, k.correct, latency);
                if k.correct {
                    if let Some(pc) = prev_char {
                        hm.record_bigram(&format!("{pc}{c}"), true, latency);
                    }
                    prev_char = Some(c);
                } else if let Some(pc) = prev_char {
                    hm.record_bigram(&format!("{pc}{c}"), false, latency);
                }
            }
            prev_press = Some(k.press_ms);
        }
        hm
    }

    /// Records a single key attempt with its latency.
    pub fn record_key(&mut self, key: char, correct: bool, latency_ms: f64) {
        let stat = self.keys.entry(key).or_default();
        stat.hits += 1;
        if !correct {
            stat.errors += 1;
        }
        stat.total_latency_ms += latency_ms;
    }

    /// Records a bigram attempt with the latency of its second key.
    pub fn record_bigram(&mut self, bigram: &str, correct: bool, latency_ms: f64) {
        let stat = self.bigrams.entry(bigram.to_string()).or_default();
        stat.hits += 1;
        if !correct {
            stat.errors += 1;
        }
        stat.total_latency_ms += latency_ms;
    }

    /// Statistics for a single key, if recorded.
    pub fn key(&self, key: char) -> Option<&UnitStat> {
        self.keys.get(&key)
    }

    /// All key statistics.
    pub fn keys(&self) -> &HashMap<char, UnitStat> {
        &self.keys
    }

    /// All bigram statistics.
    pub fn bigrams(&self) -> &HashMap<String, UnitStat> {
        &self.bigrams
    }

    /// The `n` slowest keys by average latency, descending.
    pub fn slowest_keys(&self, n: usize) -> Vec<(char, f64)> {
        let mut v: Vec<(char, f64)> = self
            .keys
            .iter()
            .map(|(&k, s)| (k, s.avg_latency_ms()))
            .collect();
        v.sort_by(|a, b| b.1.total_cmp(&a.1));
        v.truncate(n);
        v
    }

    /// The `n` most error-prone bigrams by error rate, descending. Bigrams with
    /// no errors are excluded.
    pub fn most_errored_bigrams(&self, n: usize) -> Vec<(String, f64)> {
        let mut v: Vec<(String, f64)> = self
            .bigrams
            .iter()
            .filter(|(_, s)| s.errors > 0)
            .map(|(b, s)| (b.clone(), s.error_rate()))
            .collect();
        v.sort_by(|a, b| b.1.total_cmp(&a.1));
        v.truncate(n);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_stats_compute() {
        let mut h = Heatmap::new();
        h.record_key('a', true, 100.0);
        h.record_key('a', false, 200.0);
        let s = h.key('a').unwrap();
        assert_eq!(s.hits, 2);
        assert_eq!(s.errors, 1);
        assert_eq!(s.error_rate(), 0.5);
        assert_eq!(s.avg_latency_ms(), 150.0);
    }

    #[test]
    fn health_classification() {
        let good = UnitStat {
            hits: 100,
            errors: 0,
            total_latency_ms: 5000.0,
        };
        assert_eq!(good.health(120.0), KeyHealth::Good); // 50ms avg, 0% err

        let warn = UnitStat {
            hits: 100,
            errors: 2,
            total_latency_ms: 5000.0,
        };
        assert_eq!(warn.health(120.0), KeyHealth::Warning); // 2% err

        let bad = UnitStat {
            hits: 100,
            errors: 10,
            total_latency_ms: 5000.0,
        };
        assert_eq!(bad.health(120.0), KeyHealth::Bad); // 10% err
    }

    #[test]
    fn slowest_keys_ordered() {
        let mut h = Heatmap::new();
        h.record_key('a', true, 50.0);
        h.record_key('b', true, 300.0);
        h.record_key('c', true, 150.0);
        let slow = h.slowest_keys(2);
        assert_eq!(slow[0].0, 'b');
        assert_eq!(slow[1].0, 'c');
    }

    #[test]
    fn errored_bigrams_filtered_and_ordered() {
        let mut h = Heatmap::new();
        h.record_bigram("th", true, 100.0);
        h.record_bigram("he", false, 100.0);
        let errored = h.most_errored_bigrams(5);
        assert_eq!(errored.len(), 1);
        assert_eq!(errored[0].0, "he");
    }
}
