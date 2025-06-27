//! Typing metric formulas.
//!
//! Every function here is pure and implements a definition from the TypeMaster
//! specification (Section 5) exactly. All are unit-tested against hardcoded
//! expected values (Quality rule 4). The canonical "word" length is 5 keystrokes.

/// Number of keystrokes that constitute one standardized "word".
pub const CHARS_PER_WORD: f64 = 5.0;

/// The ten fingers used in touch typing, in physical left-to-right order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Finger {
    /// Left pinky.
    LeftPinky,
    /// Left ring finger.
    LeftRing,
    /// Left middle finger.
    LeftMiddle,
    /// Left index finger.
    LeftIndex,
    /// Left thumb (space bar on most layouts).
    LeftThumb,
    /// Right thumb (space bar on most layouts).
    RightThumb,
    /// Right index finger.
    RightIndex,
    /// Right middle finger.
    RightMiddle,
    /// Right ring finger.
    RightRing,
    /// Right pinky.
    RightPinky,
}

impl Finger {
    /// Stable index in `[0, 10)` matching physical left-to-right order.
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Short two-letter label for finger-guide overlays, e.g. `"LI"`.
    pub const fn short_label(self) -> &'static str {
        match self {
            Finger::LeftPinky => "LP",
            Finger::LeftRing => "LR",
            Finger::LeftMiddle => "LM",
            Finger::LeftIndex => "LI",
            Finger::LeftThumb => "LT",
            Finger::RightThumb => "RT",
            Finger::RightIndex => "RI",
            Finger::RightMiddle => "RM",
            Finger::RightRing => "RR",
            Finger::RightPinky => "RP",
        }
    }
}

/// Maps a character to the finger that types it on a standard QWERTY layout.
///
/// Returns `None` for characters with no defined finger assignment. Uppercase
/// letters and shifted symbols resolve to the same finger as their base key
/// (the opposite-hand shift itself is enforced elsewhere).
pub fn finger_for(c: char) -> Option<Finger> {
    let lower = c.to_ascii_lowercase();
    let f = match lower {
        '`' | '1' | 'q' | 'a' | 'z' => Finger::LeftPinky,
        '2' | 'w' | 's' | 'x' => Finger::LeftRing,
        '3' | 'e' | 'd' | 'c' => Finger::LeftMiddle,
        '4' | '5' | 'r' | 't' | 'f' | 'g' | 'v' | 'b' => Finger::LeftIndex,
        ' ' => Finger::RightThumb,
        '6' | '7' | 'y' | 'u' | 'h' | 'j' | 'n' | 'm' => Finger::RightIndex,
        '8' | 'i' | 'k' | ',' => Finger::RightMiddle,
        '9' | 'o' | 'l' | '.' => Finger::RightRing,
        '0' | '-' | '=' | 'p' | '[' | ']' | ';' | '\'' | '/' | '\\' => Finger::RightPinky,
        _ => return None,
    };
    Some(f)
}

/// Converts an elapsed duration in seconds to minutes, guarding against zero.
fn minutes(elapsed_secs: f64) -> Option<f64> {
    if elapsed_secs <= 0.0 {
        None
    } else {
        Some(elapsed_secs / 60.0)
    }
}

/// Raw WPM: every keystroke counts, including errors.
///
/// `raw_wpm = (keystrokes / 5) / elapsed_minutes`. Returns `0.0` for a
/// non-positive duration.
pub fn raw_wpm(keystrokes: u32, elapsed_secs: f64) -> f64 {
    match minutes(elapsed_secs) {
        Some(m) => (f64::from(keystrokes) / CHARS_PER_WORD) / m,
        None => 0.0,
    }
}

/// Accuracy as a percentage in `[0, 100]`.
///
/// `accuracy = (total - errors) / total * 100`. Returns `100.0` when no
/// keystrokes have been recorded.
pub fn accuracy(total_keystrokes: u32, errors: u32) -> f64 {
    if total_keystrokes == 0 {
        return 100.0;
    }
    let correct = total_keystrokes.saturating_sub(errors);
    (f64::from(correct) / f64::from(total_keystrokes)) * 100.0
}

/// Net WPM: raw WPM scaled by accuracy.
///
/// `net_wpm = raw_wpm * (accuracy_pct / 100)`. `accuracy_pct` is the percentage
/// returned by [`accuracy`].
pub fn net_wpm(raw_wpm: f64, accuracy_pct: f64) -> f64 {
    raw_wpm * (accuracy_pct / 100.0)
}

/// Adjusted WPM: only fully-correct words count.
///
/// `adjusted_wpm = correct_words / elapsed_minutes`, where a word is 5
/// characters typed with zero errors. Returns `0.0` for a non-positive duration.
pub fn adjusted_wpm(correct_words: u32, elapsed_secs: f64) -> f64 {
    match minutes(elapsed_secs) {
        Some(m) => f64::from(correct_words) / m,
        None => 0.0,
    }
}

/// Population mean of a slice. Returns `0.0` for an empty slice.
pub fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// Population standard deviation of a slice. Returns `0.0` for an empty slice.
pub fn stddev(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let m = mean(values);
    let variance = values.iter().map(|v| (v - m).powi(2)).sum::<f64>() / values.len() as f64;
    variance.sqrt()
}

/// Consistency score in `[0, 100]`: rhythmic regularity of keystrokes.
///
/// Defined as `100 - coefficient_of_variation * 100`, where the coefficient of
/// variation is `stddev(intervals) / mean(intervals)`. Higher is more rhythmic.
/// Clamped to `[0, 100]`; returns `100.0` when fewer than two intervals exist or
/// the mean is zero (nothing to vary).
pub fn consistency_score(inter_key_intervals_ms: &[f64]) -> f64 {
    if inter_key_intervals_ms.len() < 2 {
        return 100.0;
    }
    let m = mean(inter_key_intervals_ms);
    if m <= 0.0 {
        return 100.0;
    }
    let cv = stddev(inter_key_intervals_ms) / m;
    (100.0 - cv * 100.0).clamp(0.0, 100.0)
}

/// Per-finger utilization as fractions in `[0, 1]`, indexed by [`Finger::index`].
///
/// Each entry is `finger_count / total`. Returns all zeros when no keystrokes
/// were recorded.
pub fn finger_utilization(counts: &[u32; 10]) -> [f64; 10] {
    let total: u32 = counts.iter().sum();
    let mut out = [0.0; 10];
    if total == 0 {
        return out;
    }
    for (o, &c) in out.iter_mut().zip(counts.iter()) {
        *o = f64::from(c) / f64::from(total);
    }
    out
}

/// Per-key latency in milliseconds: previous key *release* to this key *press*.
///
/// This is deliberately distinct from the inter-key interval (press-to-press).
/// Returns `0.0` if the timestamps are out of order.
pub fn per_key_latency(prev_release_ms: u64, this_press_ms: u64) -> f64 {
    this_press_ms.saturating_sub(prev_release_ms) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_wpm_basic() {
        // 250 keystrokes in 60s => 50 words / 1 min => 50 WPM.
        assert_eq!(raw_wpm(250, 60.0), 50.0);
    }

    #[test]
    fn raw_wpm_zero_duration_is_zero() {
        assert_eq!(raw_wpm(250, 0.0), 0.0);
    }

    #[test]
    fn accuracy_basic() {
        assert_eq!(accuracy(100, 5), 95.0);
    }

    #[test]
    fn accuracy_no_keystrokes_is_perfect() {
        assert_eq!(accuracy(0, 0), 100.0);
    }

    #[test]
    fn net_wpm_scales_by_accuracy() {
        assert_eq!(net_wpm(50.0, 95.0), 47.5);
    }

    #[test]
    fn adjusted_wpm_basic() {
        assert_eq!(adjusted_wpm(40, 60.0), 40.0);
    }

    #[test]
    fn mean_and_stddev_population() {
        assert_eq!(mean(&[100.0, 200.0]), 150.0);
        assert_eq!(stddev(&[100.0, 200.0]), 50.0);
    }

    #[test]
    fn consistency_perfect_when_even() {
        assert_eq!(consistency_score(&[100.0, 100.0, 100.0, 100.0]), 100.0);
    }

    #[test]
    fn consistency_known_value() {
        // mean 150, stddev 50, cv = 1/3 => score = 100 - 33.333... = 66.666...
        let s = consistency_score(&[100.0, 200.0]);
        assert!((s - 66.666_666_666).abs() < 1e-6, "got {s}");
    }

    #[test]
    fn consistency_single_interval_is_perfect() {
        assert_eq!(consistency_score(&[123.0]), 100.0);
    }

    #[test]
    fn finger_utilization_even() {
        let util = finger_utilization(&[1; 10]);
        for u in util {
            assert!((u - 0.1).abs() < 1e-12);
        }
    }

    #[test]
    fn finger_utilization_empty() {
        assert_eq!(finger_utilization(&[0; 10]), [0.0; 10]);
    }

    #[test]
    fn finger_map_known_keys() {
        assert_eq!(finger_for('a'), Some(Finger::LeftPinky));
        assert_eq!(finger_for('f'), Some(Finger::LeftIndex));
        assert_eq!(finger_for('j'), Some(Finger::RightIndex));
        assert_eq!(finger_for(';'), Some(Finger::RightPinky));
        assert_eq!(finger_for(' '), Some(Finger::RightThumb));
        // Uppercase resolves to the same finger as its base key.
        assert_eq!(finger_for('A'), Some(Finger::LeftPinky));
    }

    #[test]
    fn per_key_latency_basic() {
        assert_eq!(per_key_latency(1000, 1120), 120.0);
        assert_eq!(per_key_latency(1200, 1000), 0.0);
    }
}
