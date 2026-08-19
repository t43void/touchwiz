//! Curriculum model: phases, lessons, content generators, and progression.
//!
//! This implements the 0→300 WPM roadmap (specification Section 3). Each
//! [`Lesson`] carries a [`Drill`] describing how to generate its practice text,
//! a [`PassCondition`], and an ordering used for unlock gating. The concrete
//! lesson lists live in the per-phase submodules
//! ([`beginner`], [`intermediate`], [`advanced`], [`elite`]).

pub mod advanced;
pub mod beginner;
pub mod elite;
pub mod intermediate;

use std::collections::HashMap;

use rand::seq::IndexedRandom;
use rand::RngExt;
use serde::{Deserialize, Serialize};

use crate::adaptive;
use crate::corpus::Corpus;
use crate::Result;

/// Small word pool for synthesizing technical tokens (emails, URLs, paths).
const TECH_WORDS: [&str; 12] = [
    "user", "mail", "data", "index", "main", "config", "build", "node", "core", "api", "docs",
    "home",
];

/// The four pedagogical phases (specification Section 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    /// 0–30 WPM: correct muscle memory, accuracy before speed.
    Foundation,
    /// 30–70 WPM: high-frequency word chunking.
    Building,
    /// 70–150 WPM: controlled accuracy to automatic fluency.
    SpeedDevelopment,
    /// 150–300 WPM: sub-60ms inter-key intervals, zero overhead.
    Elite,
}

impl Phase {
    /// Numeric phase identifier in `[1, 4]`, used for storage and target latency.
    pub fn number(self) -> u8 {
        match self {
            Phase::Foundation => 1,
            Phase::Building => 2,
            Phase::SpeedDevelopment => 3,
            Phase::Elite => 4,
        }
    }

    /// Human-readable phase title.
    pub fn title(self) -> &'static str {
        match self {
            Phase::Foundation => "Phase 1 · Foundation",
            Phase::Building => "Phase 2 · Building",
            Phase::SpeedDevelopment => "Phase 3 · Speed Development",
            Phase::Elite => "Phase 4 · Elite",
        }
    }
}

/// Condition a session must meet to count as a pass for its lesson.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PassCondition {
    /// Minimum net WPM.
    pub required_wpm: f64,
    /// Minimum accuracy percentage.
    pub required_accuracy: f64,
    /// Target session duration in seconds (informational).
    pub duration_secs: u32,
}

/// How a lesson's practice text is generated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Drill {
    /// Pseudo-words built only from `alphabet`, lengths in `[min_len, max_len]`.
    KeySet {
        /// Allowed characters (no spaces).
        alphabet: String,
        /// Number of pseudo-words to generate.
        words: usize,
        /// Minimum word length.
        min_len: usize,
        /// Maximum word length.
        max_len: usize,
    },
    /// Repeated n-gram drills sampled from a fixed list.
    Ngrams {
        /// The n-grams to draw from.
        items: Vec<String>,
        /// How many to emit.
        count: usize,
    },
    /// Real text drawn from an embedded corpus asset.
    Corpus {
        /// Corpus asset file name (e.g. `"english_200.json"`).
        asset: String,
        /// Approximate word budget.
        words: usize,
    },
    /// Capitalized real words (shift practice), from a corpus asset.
    Capitalized {
        /// Corpus asset file name.
        asset: String,
        /// Number of words.
        words: usize,
    },
    /// Random digit groups (number-row practice).
    Numbers {
        /// Number of digit groups.
        groups: usize,
    },
    /// Symbol/punctuation tokens sampled from a fixed list.
    Symbols {
        /// The tokens to draw from.
        items: Vec<String>,
        /// How many to emit.
        count: usize,
    },
    /// Technical tokens: emails, URLs, and file paths.
    TechTokens {
        /// Number of tokens.
        count: usize,
    },
    /// Verbatim text, used for imported custom files.
    Literal {
        /// The exact text to type.
        text: String,
    },
}

impl Drill {
    /// Generates a target string for this drill using `rng`.
    pub fn generate(&self, rng: &mut impl RngExt) -> Result<String> {
        match self {
            Drill::KeySet {
                alphabet,
                words,
                min_len,
                max_len,
            } => Ok(gen_keyset(alphabet, *words, *min_len, *max_len, rng)),
            Drill::Ngrams { items, count } => Ok(sample_join(items, *count, rng)),
            Drill::Corpus { asset, words } => Ok(Corpus::load(asset)?.build_text(*words, rng)),
            Drill::Capitalized { asset, words } => {
                let corpus = Corpus::load(asset)?;
                let cap: Vec<String> = corpus
                    .sample(*words, rng)
                    .into_iter()
                    .map(|w| capitalize(&w))
                    .collect();
                Ok(cap.join(" "))
            }
            Drill::Numbers { groups } => Ok(gen_numbers(*groups, rng)),
            Drill::Symbols { items, count } => Ok(sample_join(items, *count, rng)),
            Drill::TechTokens { count } => Ok(gen_tech_tokens(*count, rng)),
            Drill::Literal { text } => Ok(text.clone()),
        }
    }

    /// A short label describing the drill's corpus type, for session records.
    pub fn corpus_type(&self) -> &'static str {
        match self {
            Drill::KeySet { .. } => "keys",
            Drill::Ngrams { .. } => "ngrams",
            Drill::Corpus { .. } => "corpus",
            Drill::Capitalized { .. } => "capitals",
            Drill::Numbers { .. } => "numbers",
            Drill::Symbols { .. } => "symbols",
            Drill::TechTokens { .. } => "tech",
            Drill::Literal { .. } => "custom",
        }
    }
}

fn gen_keyset(
    alphabet: &str,
    words: usize,
    min_len: usize,
    max_len: usize,
    rng: &mut impl RngExt,
) -> String {
    let chars: Vec<char> = alphabet.chars().filter(|c| !c.is_whitespace()).collect();
    if chars.is_empty() {
        return String::new();
    }
    let lo = min_len.max(1);
    let hi = max_len.max(lo);
    let mut out = Vec::with_capacity(words);
    for _ in 0..words {
        let len = rng.random_range(lo..=hi);
        let mut w = String::with_capacity(len);
        for _ in 0..len {
            if let Some(&c) = chars.choose(rng) {
                w.push(c);
            }
        }
        out.push(w);
    }
    out.join(" ")
}

fn sample_join(items: &[String], count: usize, rng: &mut impl RngExt) -> String {
    if items.is_empty() {
        return String::new();
    }
    (0..count)
        .filter_map(|_| items.choose(rng).cloned())
        .collect::<Vec<_>>()
        .join(" ")
}

fn gen_numbers(groups: usize, rng: &mut impl RngExt) -> String {
    (0..groups)
        .map(|_| {
            let len = rng.random_range(2..=5);
            (0..len)
                .map(|_| char::from(b'0' + rng.random_range(0..10)))
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn gen_tech_tokens(count: usize, rng: &mut impl RngExt) -> String {
    fn word(rng: &mut impl RngExt) -> &'static str {
        TECH_WORDS.choose(rng).copied().unwrap_or("user")
    }
    (0..count)
        .map(|_| match rng.random_range(0..3) {
            0 => format!("{}@{}.com", word(rng), word(rng)),
            1 => format!("https://{}.com/{}", word(rng), word(rng)),
            _ => format!("/{}/{}.rs", word(rng), word(rng)),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// A single lesson definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lesson {
    /// Stable lesson identifier, e.g. `"1.1"`.
    pub id: String,
    /// Human-readable title.
    pub title: String,
    /// One-line description of the skill being trained.
    pub goal: String,
    /// Phase this lesson belongs to.
    pub phase: Phase,
    /// How the practice text is generated.
    pub content: Drill,
    /// Pass condition for unlock progression.
    pub pass: PassCondition,
}

impl Lesson {
    /// Generates this lesson's practice text.
    pub fn generate(&self, rng: &mut impl RngExt) -> Result<String> {
        self.content.generate(rng)
    }

    /// Generates this lesson's practice text using the thread RNG.
    pub fn generate_default(&self) -> Result<String> {
        self.content.generate(&mut rand::rng())
    }

    /// A custom lesson built from imported text (no pass gating).
    pub fn custom(text: String) -> Self {
        Lesson {
            id: "custom".to_string(),
            title: "Custom Text".to_string(),
            goal: "Practice your own imported text.".to_string(),
            phase: Phase::SpeedDevelopment,
            content: Drill::Literal { text },
            pass: PassCondition {
                required_wpm: 0.0,
                required_accuracy: 0.0,
                duration_secs: 0,
            },
        }
    }

    /// The default free-typing lesson, used as a safe fallback.
    pub fn default_freetype() -> Self {
        Lesson {
            id: "free".to_string(),
            title: "Free Typing".to_string(),
            goal: "Warm up on the 200 most common words.".to_string(),
            phase: Phase::Building,
            content: Drill::Corpus {
                asset: "english_200.json".to_string(),
                words: 40,
            },
            pass: PassCondition {
                required_wpm: 30.0,
                required_accuracy: 95.0,
                duration_secs: 60,
            },
        }
    }
}

/// Convenience constructor used by the per-phase lesson lists.
#[allow(clippy::too_many_arguments)]
pub(crate) fn lesson(
    id: &str,
    title: &str,
    goal: &str,
    phase: Phase,
    content: Drill,
    wpm: f64,
    accuracy: f64,
    secs: u32,
) -> Lesson {
    Lesson {
        id: id.to_string(),
        title: title.to_string(),
        goal: goal.to_string(),
        phase,
        content,
        pass: PassCondition {
            required_wpm: wpm,
            required_accuracy: accuracy,
            duration_secs: secs,
        },
    }
}

/// Converts a slice of string literals into owned `String`s.
pub(crate) fn strs(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// Per-lesson progress, persisted across sessions.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LessonProgress {
    /// Consecutive passing sessions (resets to 0 on a fail).
    pub consecutive_passes: u32,
    /// Best net WPM achieved on this lesson.
    pub best_net_wpm: f64,
    /// Whether the lesson is completed (enough consecutive passes).
    pub completed: bool,
}

/// The result of evaluating a finished session against its lesson.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Outcome {
    /// Whether the session met the pass condition.
    pub passed: bool,
    /// Consecutive passes after this session.
    pub consecutive_passes: u32,
    /// Whether the lesson is now completed.
    pub completed: bool,
    /// Whether this session is what completed the lesson (unlocks the next).
    pub newly_completed: bool,
}

/// The full ordered curriculum.
#[derive(Debug, Clone)]
pub struct Curriculum {
    lessons: Vec<Lesson>,
}

impl Default for Curriculum {
    fn default() -> Self {
        Self::new()
    }
}

impl Curriculum {
    /// Builds the complete curriculum across all four phases.
    pub fn new() -> Self {
        let mut lessons = Vec::new();
        lessons.extend(beginner::lessons());
        lessons.extend(intermediate::lessons());
        lessons.extend(advanced::lessons());
        lessons.extend(elite::lessons());
        Curriculum { lessons }
    }

    /// All lessons in order.
    pub fn lessons(&self) -> &[Lesson] {
        &self.lessons
    }

    /// Number of lessons.
    pub fn len(&self) -> usize {
        self.lessons.len()
    }

    /// Whether the curriculum is empty (never, but satisfies clippy).
    pub fn is_empty(&self) -> bool {
        self.lessons.is_empty()
    }

    /// The lesson at `index`, if any.
    pub fn get(&self, index: usize) -> Option<&Lesson> {
        self.lessons.get(index)
    }

    /// Index of the lesson with `id`, if present.
    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.lessons.iter().position(|l| l.id == id)
    }

    /// Whether the lesson at `index` is unlocked: the first lesson is always
    /// unlocked; later lessons require the previous one to be completed.
    pub fn is_unlocked(&self, index: usize, progress: &HashMap<String, LessonProgress>) -> bool {
        if index == 0 {
            return true;
        }
        match self.lessons.get(index - 1) {
            Some(prev) => progress.get(&prev.id).map(|p| p.completed).unwrap_or(false),
            None => false,
        }
    }

    /// The next unlocked lesson index after `from`, if any.
    pub fn next_unlocked(
        &self,
        from: usize,
        progress: &HashMap<String, LessonProgress>,
    ) -> Option<usize> {
        ((from + 1)..self.lessons.len()).find(|&i| self.is_unlocked(i, progress))
    }
}

/// Records a finished session against a lesson, updating `progress` in place and
/// returning the [`Outcome`]. A pass requires both the WPM and accuracy
/// thresholds; [`adaptive::PASSES_TO_UNLOCK`] consecutive passes complete it.
pub fn evaluate(
    progress: &mut HashMap<String, LessonProgress>,
    lesson: &Lesson,
    net_wpm: f64,
    accuracy: f64,
) -> Outcome {
    let entry = progress.entry(lesson.id.clone()).or_default();
    let was_completed = entry.completed;

    let passed = adaptive::is_pass(
        net_wpm,
        accuracy,
        lesson.pass.required_wpm,
        lesson.pass.required_accuracy,
    );
    if passed {
        entry.consecutive_passes += 1;
        if net_wpm > entry.best_net_wpm {
            entry.best_net_wpm = net_wpm;
        }
    } else {
        entry.consecutive_passes = 0;
    }
    if adaptive::is_unlocked(entry.consecutive_passes) {
        entry.completed = true;
    }

    Outcome {
        passed,
        consecutive_passes: entry.consecutive_passes,
        completed: entry.completed,
        newly_completed: entry.completed && !was_completed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn phase_numbers() {
        assert_eq!(Phase::Foundation.number(), 1);
        assert_eq!(Phase::Elite.number(), 4);
    }

    #[test]
    fn curriculum_spans_all_phases() {
        let c = Curriculum::new();
        assert!(c.len() >= 24, "expected a full curriculum, got {}", c.len());
        for phase in [
            Phase::Foundation,
            Phase::Building,
            Phase::SpeedDevelopment,
            Phase::Elite,
        ] {
            assert!(
                c.lessons().iter().any(|l| l.phase == phase),
                "missing lessons for {phase:?}"
            );
        }
    }

    #[test]
    fn every_lesson_generates_nonempty_text() {
        let c = Curriculum::new();
        let mut rng = StdRng::seed_from_u64(1);
        for l in c.lessons() {
            let text = l.generate(&mut rng).expect("generate");
            assert!(!text.trim().is_empty(), "empty text for lesson {}", l.id);
        }
    }

    #[test]
    fn keyset_only_uses_allowed_characters() {
        let drill = Drill::KeySet {
            alphabet: "asdfjkl;".to_string(),
            words: 20,
            min_len: 2,
            max_len: 5,
        };
        let mut rng = StdRng::seed_from_u64(7);
        let text = drill.generate(&mut rng).unwrap();
        for c in text.chars() {
            assert!(c == ' ' || "asdfjkl;".contains(c), "stray char {c:?}");
        }
    }

    #[test]
    fn unlock_requires_previous_completion() {
        let c = Curriculum::new();
        let mut progress: HashMap<String, LessonProgress> = HashMap::new();
        assert!(c.is_unlocked(0, &progress));
        assert!(!c.is_unlocked(1, &progress));

        let first_id = c.lessons()[0].id.clone();
        progress.insert(
            first_id,
            LessonProgress {
                consecutive_passes: 3,
                best_net_wpm: 40.0,
                completed: true,
            },
        );
        assert!(c.is_unlocked(1, &progress));
    }

    #[test]
    fn evaluate_completes_after_three_passes() {
        let c = Curriculum::new();
        let lesson = &c.lessons()[0];
        let req = lesson.pass;
        let mut progress = HashMap::new();

        let pass_wpm = req.required_wpm + 5.0;
        let pass_acc = req.required_accuracy + 1.0;

        let o1 = evaluate(&mut progress, lesson, pass_wpm, pass_acc);
        assert!(o1.passed && !o1.completed && o1.consecutive_passes == 1);
        evaluate(&mut progress, lesson, pass_wpm, pass_acc);
        let o3 = evaluate(&mut progress, lesson, pass_wpm, pass_acc);
        assert!(o3.completed && o3.newly_completed);

        // A failing run resets the streak but the lesson stays completed.
        let o4 = evaluate(&mut progress, lesson, 0.0, 0.0);
        assert!(!o4.passed);
        assert_eq!(o4.consecutive_passes, 0);
        assert!(o4.completed);
    }
}
