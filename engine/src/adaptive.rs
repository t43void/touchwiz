//! Adaptive scheduling: a modified SM-2 (SuperMemo 2) algorithm tuned for
//! typing, plus word selection, progression gating, and regression detection.
//!
//! This implements TypeMaster specification Section 4 exactly. Each trainable
//! unit (key, bigram, or word) is a [`Card`] whose `easiness_factor` and review
//! `interval` adapt to measured latency and error rate.

use rand::distr::weighted::WeightedIndex;
use rand::distr::Distribution;
use rand::seq::IndexedRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Starting easiness factor for a fresh card (SM-2 default).
pub const DEFAULT_EASINESS: f32 = 2.5;
/// Lower clamp for the easiness factor.
pub const MIN_EASINESS: f32 = 1.3;
/// Upper clamp for the easiness factor.
pub const MAX_EASINESS: f32 = 4.0;
/// Milliseconds in one day, used to compute the next-review timestamp.
const MS_PER_DAY: u64 = 86_400_000;
/// Consecutive passing sessions required to unlock the next lesson.
pub const PASSES_TO_UNLOCK: u32 = 3;
/// Fractional drop from the 7-day average that triggers a consolidation session.
pub const REGRESSION_THRESHOLD: f64 = 0.15;

/// Phase-appropriate target inter-key latency in milliseconds (Section 4).
///
/// Phase 1 trains for control (300ms); Phase 4 targets elite speed (60ms).
pub fn target_latency_ms(phase: u8) -> f32 {
    match phase {
        1 => 300.0,
        2 => 200.0,
        3 => 120.0,
        _ => 60.0,
    }
}

/// Performance score in `[0, ∞)` combining accuracy and speed (Section 4).
///
/// `(1 - error_rate) * (target_latency / avg_latency)`. A value of `1.0` means
/// on-target; above `1.0` means faster than target. `avg_latency_ms <= 0` is
/// treated as on-target to avoid division by zero.
pub fn performance_score(error_rate: f32, avg_latency_ms: f32, target_latency_ms: f32) -> f32 {
    let accuracy = (1.0 - error_rate).clamp(0.0, 1.0);
    let ratio = if avg_latency_ms <= 0.0 {
        1.0
    } else {
        target_latency_ms / avg_latency_ms
    };
    accuracy * ratio
}

/// A spaced-repetition card for a single trainable unit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Card {
    /// The unit this card trains (a key, bigram, or word).
    pub unit: String,
    /// SM-2 easiness factor, clamped to `[MIN_EASINESS, MAX_EASINESS]`.
    pub easiness_factor: f32,
    /// Days until the next scheduled review.
    pub interval: u32,
    /// Number of successful reviews so far.
    pub repetitions: u32,
    /// Rolling average inter-key latency in milliseconds.
    pub avg_latency_ms: f32,
    /// Rolling error rate in `[0, 1]`.
    pub error_rate: f32,
    /// Unix-epoch milliseconds of the most recent review.
    pub last_seen: u64,
}

impl Card {
    /// Creates a fresh, never-reviewed card for `unit`.
    pub fn new(unit: impl Into<String>) -> Self {
        Card {
            unit: unit.into(),
            easiness_factor: DEFAULT_EASINESS,
            interval: 0,
            repetitions: 0,
            avg_latency_ms: 999.0,
            error_rate: 1.0,
            last_seen: 0,
        }
    }

    /// Applies one review with freshly measured `error_rate` and
    /// `avg_latency_ms`, updating easiness, interval, and repetitions per the
    /// SM-2 rules in Section 4.
    pub fn review(
        &mut self,
        error_rate: f32,
        avg_latency_ms: f32,
        target_latency_ms: f32,
        now_ms: u64,
    ) {
        self.error_rate = error_rate;
        self.avg_latency_ms = avg_latency_ms;

        let perf = performance_score(error_rate, avg_latency_ms, target_latency_ms);
        self.easiness_factor =
            (self.easiness_factor + 0.1 - (1.0 - perf) * 0.8).clamp(MIN_EASINESS, MAX_EASINESS);

        self.interval = match self.repetitions {
            0 => 1,
            1 => 6,
            _ => (self.interval as f32 * self.easiness_factor).round() as u32,
        };
        self.repetitions += 1;
        self.last_seen = now_ms;
    }

    /// Whether this card is due for review at `now_ms`.
    pub fn is_due(&self, now_ms: u64) -> bool {
        now_ms >= self.next_review()
    }

    /// Unix-epoch milliseconds at which this card next becomes due.
    pub fn next_review(&self) -> u64 {
        self.last_seen + u64::from(self.interval) * MS_PER_DAY
    }

    /// Whether the card counts as "known": fast and accurate enough that it no
    /// longer needs to be drilled as new material.
    pub fn is_known(&self, latency_threshold_ms: f32) -> bool {
        self.avg_latency_ms < latency_threshold_ms && self.error_rate < 0.03
    }
}

/// Chooses one entry from `pool`, biased toward entries containing `weak_keys`.
fn choose_weighted(pool: &[String], weak_keys: &[char], rng: &mut impl Rng) -> Option<String> {
    if pool.is_empty() {
        return None;
    }
    if weak_keys.is_empty() {
        return pool.choose(rng).cloned();
    }
    let weights: Vec<u32> = pool
        .iter()
        .map(|w| 1 + weak_keys.iter().filter(|&&k| w.contains(k)).count() as u32 * 3)
        .collect();
    match WeightedIndex::new(&weights) {
        Ok(dist) => Some(pool[dist.sample(rng)].clone()),
        Err(_) => pool.choose(rng).cloned(),
    }
}

/// Builds a session word list of length `count` from `known` and `unknown`
/// pools (Section 4 word selection).
///
/// Roughly 70% of words come from `known` and 30% from `unknown`; unknown
/// positions are evenly spaced so no more than two unknown words ever appear
/// consecutively, and unknown picks are weighted toward `weak_keys`. Sampling is
/// with replacement, so pool size never limits `count`.
pub fn select_session_words(
    known: &[String],
    unknown: &[String],
    count: usize,
    weak_keys: &[char],
    rng: &mut impl Rng,
) -> Vec<String> {
    if count == 0 {
        return Vec::new();
    }
    let n_unknown = if unknown.is_empty() {
        0
    } else if known.is_empty() {
        count
    } else {
        (count as f32 * 0.3).round() as usize
    };

    let mut out = Vec::with_capacity(count);
    let mut placed_unknown = 0usize;
    // Bresenham-style even spacing guarantees exactly `n_unknown` unknown slots
    // with maximal separation, satisfying the "no >2 consecutive" rule.
    for i in 0..count {
        let cur = (i + 1) * n_unknown / count;
        let is_unknown = cur > placed_unknown;
        let word = if is_unknown {
            placed_unknown = cur;
            choose_weighted(unknown, weak_keys, rng)
        } else {
            known
                .choose(rng)
                .cloned()
                .or_else(|| unknown.choose(rng).cloned())
        };
        if let Some(w) = word {
            out.push(w);
        }
    }
    out
}

/// Whether a lesson's pass condition is met for one session.
pub fn is_pass(net_wpm: f64, accuracy_pct: f64, required_wpm: f64, required_accuracy: f64) -> bool {
    net_wpm >= required_wpm && accuracy_pct >= required_accuracy
}

/// Whether enough consecutive passes have accrued to unlock the next lesson.
pub fn is_unlocked(consecutive_passes: u32) -> bool {
    consecutive_passes >= PASSES_TO_UNLOCK
}

/// Whether recent performance has regressed enough to warrant a consolidation
/// session: a drop of more than [`REGRESSION_THRESHOLD`] below the 7-day average.
pub fn is_regression(recent_net_wpm: f64, seven_day_avg_wpm: f64) -> bool {
    seven_day_avg_wpm > 0.0 && recent_net_wpm < seven_day_avg_wpm * (1.0 - REGRESSION_THRESHOLD)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn new_card_defaults() {
        let c = Card::new("th");
        assert_eq!(c.easiness_factor, 2.5);
        assert_eq!(c.repetitions, 0);
        assert_eq!(c.interval, 0);
    }

    #[test]
    fn performance_score_on_target_is_one() {
        assert_eq!(performance_score(0.0, 300.0, 300.0), 1.0);
    }

    #[test]
    fn performance_score_fast_and_accurate() {
        // no errors, twice as fast as target => 2.0
        assert_eq!(performance_score(0.0, 150.0, 300.0), 2.0);
    }

    #[test]
    fn review_perfect_raises_easiness_and_sets_intervals() {
        let mut c = Card::new("e");
        // perf == 1 => easiness += 0.1
        c.review(0.0, 300.0, 300.0, 1_000);
        assert!((c.easiness_factor - 2.6).abs() < 1e-6);
        assert_eq!(c.interval, 1);
        assert_eq!(c.repetitions, 1);

        c.review(0.0, 300.0, 300.0, 2_000);
        assert_eq!(c.interval, 6);
        assert_eq!(c.repetitions, 2);

        // third review: interval = round(prev_interval * easiness), using the
        // easiness updated by this same review.
        c.review(0.0, 300.0, 300.0, 3_000);
        assert_eq!(c.interval, (6.0 * c.easiness_factor).round() as u32);
    }

    #[test]
    fn review_poor_lowers_easiness_clamped() {
        let mut c = Card::new("z");
        // Drive easiness to the floor with repeated terrible performance.
        for t in 0..50 {
            c.review(1.0, 999.0, 60.0, t);
        }
        assert_eq!(c.easiness_factor, MIN_EASINESS);
    }

    #[test]
    fn review_excellent_clamps_at_ceiling() {
        let mut c = Card::new("a");
        for t in 0..50 {
            c.review(0.0, 10.0, 300.0, t); // wildly above target
        }
        assert_eq!(c.easiness_factor, MAX_EASINESS);
    }

    #[test]
    fn known_classification() {
        let mut c = Card::new("f");
        c.avg_latency_ms = 90.0;
        c.error_rate = 0.0;
        assert!(c.is_known(120.0));
        c.error_rate = 0.10;
        assert!(!c.is_known(120.0));
    }

    #[test]
    fn word_selection_respects_70_30_split() {
        let known: Vec<String> = (0..50).map(|i| format!("k{i}")).collect();
        let unknown: Vec<String> = (0..50).map(|i| format!("u{i}")).collect();
        let mut rng = StdRng::seed_from_u64(99);
        let words = select_session_words(&known, &unknown, 10, &[], &mut rng);
        assert_eq!(words.len(), 10);
        let unknown_count = words.iter().filter(|w| w.starts_with('u')).count();
        assert_eq!(unknown_count, 3, "expected 30% unknown");
    }

    #[test]
    fn word_selection_no_three_consecutive_unknown() {
        let known: Vec<String> = (0..50).map(|i| format!("k{i}")).collect();
        let unknown: Vec<String> = (0..50).map(|i| format!("u{i}")).collect();
        let mut rng = StdRng::seed_from_u64(3);
        let words = select_session_words(&known, &unknown, 40, &[], &mut rng);
        let mut run = 0;
        for w in &words {
            if w.starts_with('u') {
                run += 1;
                assert!(run <= 2, "more than 2 consecutive unknown words");
            } else {
                run = 0;
            }
        }
    }

    #[test]
    fn gating_and_regression() {
        assert!(is_pass(55.0, 96.0, 50.0, 95.0));
        assert!(!is_pass(45.0, 96.0, 50.0, 95.0));
        assert!(is_unlocked(3));
        assert!(!is_unlocked(2));
        assert!(is_regression(80.0, 100.0)); // 20% drop
        assert!(!is_regression(90.0, 100.0)); // 10% drop
    }
}
