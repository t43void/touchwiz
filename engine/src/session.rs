//! Per-session state machine and keystroke recording.
//!
//! A [`Session`] owns the target text (as grapheme clusters), the running tally
//! of keystrokes, and the timing buffers needed to compute live and final
//! metrics. All time is supplied by the caller as Unix-epoch milliseconds so the
//! engine stays free of clock I/O and is deterministically testable.

use unicode_segmentation::UnicodeSegmentation;

use crate::metrics::{self, Finger};
use crate::{Error, Result};

/// Lifecycle of a typing session. Transitions are enforced by [`Session`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Created but not yet started; no timing has begun.
    Idle,
    /// Actively accepting keystrokes; the clock is running.
    Active,
    /// Temporarily halted; elapsed time excludes the paused span.
    Paused,
    /// Completed; no further keystrokes are accepted.
    Finished,
}

/// A single recorded keystroke.
#[derive(Debug, Clone)]
pub struct Keystroke {
    /// The grapheme the user actually typed.
    pub typed: String,
    /// The grapheme that was expected at this position.
    pub expected: String,
    /// Whether `typed` matched `expected`.
    pub correct: bool,
    /// Press time in Unix-epoch milliseconds.
    pub press_ms: u64,
    /// Finger responsible for the expected key, when known.
    pub finger: Option<Finger>,
}

/// A typing session over a fixed target text.
#[derive(Debug, Clone)]
pub struct Session {
    state: SessionState,
    target: Vec<String>,
    cursor: usize,
    keystrokes: Vec<Keystroke>,
    finger_counts: [u32; 10],
    errors: u32,
    start_ms: Option<u64>,
    end_ms: Option<u64>,
    /// Total time spent paused, in milliseconds, excluded from elapsed.
    paused_accum_ms: u64,
    paused_at_ms: Option<u64>,
    last_press_ms: Option<u64>,
    inter_key_intervals_ms: Vec<f64>,
}

impl Session {
    /// Creates an idle session from `target`, split into grapheme clusters
    /// (Quality rule 13). Whitespace is preserved as part of the target.
    pub fn new(target: &str) -> Self {
        let target: Vec<String> = target.graphemes(true).map(str::to_string).collect();
        Session {
            state: SessionState::Idle,
            target,
            cursor: 0,
            keystrokes: Vec::new(),
            finger_counts: [0; 10],
            errors: 0,
            start_ms: None,
            end_ms: None,
            paused_accum_ms: 0,
            paused_at_ms: None,
            last_press_ms: None,
            inter_key_intervals_ms: Vec::new(),
        }
    }

    /// Current lifecycle state.
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// The target text as grapheme clusters.
    pub fn target(&self) -> &[String] {
        &self.target
    }

    /// Index of the next grapheme to be typed.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Total graphemes in the target.
    pub fn total_chars(&self) -> usize {
        self.target.len()
    }

    /// All recorded keystrokes in order.
    pub fn keystrokes(&self) -> &[Keystroke] {
        &self.keystrokes
    }

    /// Number of incorrect keystrokes recorded so far.
    pub fn error_count(&self) -> u32 {
        self.errors
    }

    /// Begins the session. Errors if not currently [`SessionState::Idle`].
    pub fn start(&mut self, now_ms: u64) -> Result<()> {
        if self.state != SessionState::Idle {
            return Err(Error::InvalidTransition("start requires Idle"));
        }
        self.state = SessionState::Active;
        self.start_ms = Some(now_ms);
        Ok(())
    }

    /// Pauses an active session. Errors if not currently [`SessionState::Active`].
    pub fn pause(&mut self, now_ms: u64) -> Result<()> {
        if self.state != SessionState::Active {
            return Err(Error::InvalidTransition("pause requires Active"));
        }
        self.state = SessionState::Paused;
        self.paused_at_ms = Some(now_ms);
        Ok(())
    }

    /// Resumes a paused session, accumulating the paused span so it is excluded
    /// from elapsed time. Errors if not currently [`SessionState::Paused`].
    pub fn resume(&mut self, now_ms: u64) -> Result<()> {
        if self.state != SessionState::Paused {
            return Err(Error::InvalidTransition("resume requires Paused"));
        }
        if let Some(at) = self.paused_at_ms.take() {
            self.paused_accum_ms += now_ms.saturating_sub(at);
        }
        // Avoid charging the pause gap as an inter-key interval.
        self.last_press_ms = None;
        self.state = SessionState::Active;
        Ok(())
    }

    /// Records a typed grapheme against the current cursor position and advances
    /// the cursor. Errors if the session is not active. Returns whether the
    /// keystroke was correct. Keystrokes past the end of the target are ignored.
    pub fn record(&mut self, typed: &str, now_ms: u64) -> Result<bool> {
        if self.state != SessionState::Active {
            return Err(Error::InvalidTransition("record requires Active"));
        }
        let Some(expected) = self.target.get(self.cursor).cloned() else {
            return Ok(true);
        };

        let correct = typed == expected;
        let finger = expected.chars().next().and_then(metrics::finger_for);

        if !correct {
            self.errors += 1;
        }
        if let Some(f) = finger {
            self.finger_counts[f.index()] += 1;
        }
        if let Some(prev) = self.last_press_ms {
            self.inter_key_intervals_ms
                .push(now_ms.saturating_sub(prev) as f64);
        }
        self.last_press_ms = Some(now_ms);

        self.keystrokes.push(Keystroke {
            typed: typed.to_string(),
            expected,
            correct,
            press_ms: now_ms,
            finger,
        });
        self.cursor += 1;

        if self.cursor >= self.target.len() {
            self.finish_internal(now_ms);
        }
        Ok(correct)
    }

    /// Marks the session finished. Idempotent once finished.
    pub fn finish(&mut self, now_ms: u64) {
        if self.state != SessionState::Finished {
            self.finish_internal(now_ms);
        }
    }

    fn finish_internal(&mut self, now_ms: u64) {
        if self.state == SessionState::Paused {
            if let Some(at) = self.paused_at_ms.take() {
                self.paused_accum_ms += now_ms.saturating_sub(at);
            }
        }
        self.state = SessionState::Finished;
        self.end_ms = Some(now_ms);
    }

    /// Elapsed active milliseconds, excluding paused spans, as of `now_ms`.
    pub fn elapsed_ms(&self, now_ms: u64) -> u64 {
        let Some(start) = self.start_ms else {
            return 0;
        };
        let reference = self.end_ms.unwrap_or(now_ms);
        let gross = reference.saturating_sub(start);
        gross.saturating_sub(self.paused_accum_ms)
    }

    /// Elapsed active time in seconds as of `now_ms`.
    pub fn elapsed_secs(&self, now_ms: u64) -> f64 {
        self.elapsed_ms(now_ms) as f64 / 1000.0
    }

    /// Total keystrokes recorded.
    pub fn total_keystrokes(&self) -> u32 {
        self.keystrokes.len() as u32
    }

    /// Number of fully-correct 5-character words, used for adjusted WPM.
    fn correct_words(&self) -> u32 {
        let correct = self.total_keystrokes().saturating_sub(self.errors);
        (f64::from(correct) / metrics::CHARS_PER_WORD).floor() as u32
    }

    /// Live raw WPM as of `now_ms`.
    pub fn raw_wpm(&self, now_ms: u64) -> f64 {
        metrics::raw_wpm(self.total_keystrokes(), self.elapsed_secs(now_ms))
    }

    /// Live accuracy percentage.
    pub fn accuracy(&self) -> f64 {
        metrics::accuracy(self.total_keystrokes(), self.errors)
    }

    /// Live net WPM as of `now_ms`.
    pub fn net_wpm(&self, now_ms: u64) -> f64 {
        metrics::net_wpm(self.raw_wpm(now_ms), self.accuracy())
    }

    /// Live adjusted WPM as of `now_ms`.
    pub fn adjusted_wpm(&self, now_ms: u64) -> f64 {
        metrics::adjusted_wpm(self.correct_words(), self.elapsed_secs(now_ms))
    }

    /// Net WPM over the trailing `window_ms` window ending at `now_ms`.
    ///
    /// Used for the smoothed live readout (Section 5: rolling 10-second window).
    pub fn windowed_net_wpm(&self, now_ms: u64, window_ms: u64) -> f64 {
        let cutoff = now_ms.saturating_sub(window_ms);
        let mut count = 0u32;
        let mut errors = 0u32;
        for k in &self.keystrokes {
            if k.press_ms >= cutoff {
                count += 1;
                if !k.correct {
                    errors += 1;
                }
            }
        }
        if count == 0 {
            return 0.0;
        }
        let secs = (now_ms.saturating_sub(cutoff)) as f64 / 1000.0;
        let raw = metrics::raw_wpm(count, secs);
        metrics::net_wpm(raw, metrics::accuracy(count, errors))
    }

    /// Live consistency score from inter-key intervals.
    pub fn consistency(&self) -> f64 {
        metrics::consistency_score(&self.inter_key_intervals_ms)
    }

    /// Per-finger keystroke counts indexed by [`Finger::index`].
    pub fn finger_counts(&self) -> &[u32; 10] {
        &self.finger_counts
    }

    /// Recorded inter-key intervals in milliseconds.
    pub fn inter_key_intervals(&self) -> &[f64] {
        &self.inter_key_intervals_ms
    }

    /// Session start time, if started.
    pub fn started_at(&self) -> Option<u64> {
        self.start_ms
    }

    /// Session end time, if finished.
    pub fn ended_at(&self) -> Option<u64> {
        self.end_ms
    }

    /// Per-word statistics for spaced-repetition scheduling.
    ///
    /// Words are runs of non-space expected graphemes. For each word, the error
    /// rate and average inter-key latency are computed from its keystrokes, which
    /// the adaptive scheduler turns into a [`crate::adaptive::Card`] review.
    pub fn word_stats(&self) -> Vec<WordStat> {
        let mut out = Vec::new();
        let mut word = String::new();
        let mut total = 0u32;
        let mut errors = 0u32;
        let mut lat_sum = 0f64;
        let mut lat_n = 0u32;
        let mut prev_press: Option<u64> = None;

        let mut flush = |word: &mut String,
                         total: &mut u32,
                         errors: &mut u32,
                         lat_sum: &mut f64,
                         lat_n: &mut u32| {
            if !word.is_empty() && *total > 0 {
                out.push(WordStat {
                    word: std::mem::take(word),
                    error_rate: *errors as f32 / *total as f32,
                    avg_latency_ms: if *lat_n > 0 {
                        (*lat_sum / f64::from(*lat_n)) as f32
                    } else {
                        0.0
                    },
                });
            } else {
                word.clear();
            }
            *total = 0;
            *errors = 0;
            *lat_sum = 0.0;
            *lat_n = 0;
        };

        for k in &self.keystrokes {
            if let Some(p) = prev_press {
                lat_sum += k.press_ms.saturating_sub(p) as f64;
                lat_n += 1;
            }
            prev_press = Some(k.press_ms);

            if k.expected == " " {
                flush(&mut word, &mut total, &mut errors, &mut lat_sum, &mut lat_n);
                continue;
            }
            total += 1;
            if !k.correct {
                errors += 1;
            }
            word.push_str(&k.expected);
        }
        flush(&mut word, &mut total, &mut errors, &mut lat_sum, &mut lat_n);
        out
    }
}

/// Aggregated typing statistics for a single word within a session.
#[derive(Debug, Clone, PartialEq)]
pub struct WordStat {
    /// The intended word (from expected graphemes).
    pub word: String,
    /// Error rate over the word's keystrokes, in `[0, 1]`.
    pub error_rate: f32,
    /// Average inter-key latency over the word's keystrokes, in milliseconds.
    pub avg_latency_ms: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_transitions() {
        let mut s = Session::new("ab");
        assert_eq!(s.state(), SessionState::Idle);
        assert!(s.record("a", 10).is_err()); // cannot record before start
        s.start(0).unwrap();
        assert_eq!(s.state(), SessionState::Active);
        assert!(s.start(0).is_err()); // cannot start twice
        s.record("a", 100).unwrap();
        s.record("b", 200).unwrap();
        // Auto-finishes when the target is exhausted.
        assert_eq!(s.state(), SessionState::Finished);
    }

    #[test]
    fn records_correctness_and_errors() {
        let mut s = Session::new("fj");
        s.start(0).unwrap();
        assert!(s.record("f", 100).unwrap());
        assert!(!s.record("x", 200).unwrap()); // wrong key
        assert_eq!(s.error_count(), 1);
        assert_eq!(s.total_keystrokes(), 2);
        assert_eq!(s.accuracy(), 50.0);
    }

    #[test]
    fn pause_excludes_elapsed() {
        let mut s = Session::new("abcd");
        s.start(0).unwrap();
        s.record("a", 1000).unwrap();
        s.pause(2000).unwrap();
        s.resume(5000).unwrap(); // 3s paused
        s.record("b", 6000).unwrap();
        // gross = 6000, paused 3000 => elapsed 3000ms.
        assert_eq!(s.elapsed_ms(6000), 3000);
    }

    #[test]
    fn inter_key_intervals_tracked() {
        let mut s = Session::new("abc");
        s.start(0).unwrap();
        s.record("a", 100).unwrap();
        s.record("b", 250).unwrap();
        // first keystroke has no predecessor; one interval of 150ms recorded.
        assert_eq!(s.inter_key_intervals(), &[150.0]);
    }

    #[test]
    fn word_stats_split_on_spaces() {
        let mut s = Session::new("ab cd");
        s.start(0).unwrap();
        s.record("a", 100).unwrap();
        s.record("x", 200).unwrap(); // wrong: expected 'b'
        s.record(" ", 300).unwrap();
        s.record("c", 400).unwrap();
        s.record("d", 500).unwrap();
        let stats = s.word_stats();
        assert_eq!(stats.len(), 2);
        assert_eq!(stats[0].word, "ab");
        assert_eq!(stats[0].error_rate, 0.5);
        assert_eq!(stats[1].word, "cd");
        assert_eq!(stats[1].error_rate, 0.0);
    }

    #[test]
    fn finger_counts_accumulate() {
        let mut s = Session::new("aj");
        s.start(0).unwrap();
        s.record("a", 100).unwrap();
        s.record("j", 200).unwrap();
        assert_eq!(s.finger_counts()[Finger::LeftPinky.index()], 1);
        assert_eq!(s.finger_counts()[Finger::RightIndex.index()], 1);
    }
}
