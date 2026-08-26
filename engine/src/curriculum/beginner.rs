//! Phase 1 — Foundation (0–30 WPM): correct muscle memory, accuracy first.
//!
//! After home-row anchors and the most common letters, each finger's remaining
//! keys are trained before the full alphabet — so motor patterns settle before
//! everything is thrown in together.

use super::{lesson, strs, Drill, Lesson, Phase::Foundation};

/// Keys unlocked through lesson 1.3 (home row + e i + t n o h r).
const AFTER_TNOHR: &str = "asdfjkl;eitnohr";

/// The Foundation-phase lessons (1.1 – 1.12).
pub fn lessons() -> Vec<Lesson> {
    vec![
        lesson(
            "1.1",
            "Home Row",
            "Anchor your fingers on a s d f  j k l ;",
            Foundation,
            Drill::KeySet {
                alphabet: "asdfjkl;".to_string(),
                words: 30,
                min_len: 2,
                max_len: 5,
            },
            10.0,
            98.0,
            120,
        ),
        lesson(
            "1.2",
            "Home Row + E I",
            "Add the two highest-frequency vowels.",
            Foundation,
            Drill::KeySet {
                alphabet: "asdfjkl;ei".to_string(),
                words: 32,
                min_len: 2,
                max_len: 5,
            },
            12.0,
            98.0,
            120,
        ),
        lesson(
            "1.3",
            "Top Letters T N O H R",
            "Reach for the most common English consonants.",
            Foundation,
            Drill::KeySet {
                alphabet: AFTER_TNOHR.to_string(),
                words: 34,
                min_len: 2,
                max_len: 6,
            },
            14.0,
            97.0,
            120,
        ),
        lesson(
            "1.4",
            "Left Index Reach",
            "Train the left index on g b v — keep home anchors.",
            Foundation,
            Drill::KeySet {
                alphabet: format!("{AFTER_TNOHR}gbv"),
                words: 34,
                min_len: 2,
                max_len: 6,
            },
            14.0,
            97.0,
            120,
        ),
        lesson(
            "1.5",
            "Right Index Reach",
            "Train the right index on y u m.",
            Foundation,
            Drill::KeySet {
                alphabet: format!("{AFTER_TNOHR}gbvyum"),
                words: 34,
                min_len: 2,
                max_len: 6,
            },
            15.0,
            97.0,
            120,
        ),
        lesson(
            "1.6",
            "Middle & Ring Reach",
            "Left middle c, left ring w x — stretch without lifting anchors.",
            Foundation,
            Drill::KeySet {
                alphabet: format!("{AFTER_TNOHR}gbvyumcwx"),
                words: 34,
                min_len: 2,
                max_len: 6,
            },
            15.0,
            96.0,
            120,
        ),
        lesson(
            "1.7",
            "Pinky Reach",
            "Pinkies on q z p — the awkward keys, one finger at a time.",
            Foundation,
            Drill::KeySet {
                alphabet: format!("{AFTER_TNOHR}gbvyumcwxqzp"),
                words: 34,
                min_len: 2,
                max_len: 6,
            },
            15.0,
            96.0,
            120,
        ),
        lesson(
            "1.8",
            "Full Alphabet",
            "Every letter together — fingers already know their homes.",
            Foundation,
            Drill::KeySet {
                alphabet: "abcdefghijklmnopqrstuvwxyz".to_string(),
                words: 34,
                min_len: 3,
                max_len: 6,
            },
            16.0,
            96.0,
            120,
        ),
        lesson(
            "1.9",
            "Finger Drill Bigrams",
            "Burn in the highest-frequency two-key sequences.",
            Foundation,
            Drill::Ngrams {
                items: strs(&[
                    "th", "he", "in", "er", "an", "re", "on", "at", "en", "nd", "ti", "es", "or",
                    "te", "of", "ed", "is", "it", "al", "ar",
                ]),
                count: 40,
            },
            18.0,
            96.0,
            120,
        ),
        lesson(
            "1.10",
            "Number Row",
            "Stretch to the digits, left to right.",
            Foundation,
            Drill::Numbers { groups: 24 },
            14.0,
            96.0,
            120,
        ),
        lesson(
            "1.11",
            "Shift & Capitals",
            "Use the opposite-hand shift for capital letters.",
            Foundation,
            Drill::Capitalized {
                asset: "english_200.json".to_string(),
                words: 30,
            },
            18.0,
            96.0,
            120,
        ),
        lesson(
            "1.12",
            "Punctuation",
            "Periods, commas, apostrophes, and question marks.",
            Foundation,
            Drill::Symbols {
                items: strs(&[
                    "it's", "don't", "yes,", "no.", "who?", "she's", "stop.", "go,", "why?",
                    "can't",
                ]),
                count: 28,
            },
            18.0,
            96.0,
            120,
        ),
    ]
}
