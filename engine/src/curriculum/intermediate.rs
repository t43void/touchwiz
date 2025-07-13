//! Phase 2 — Building (30–70 WPM): eliminate hesitation via word chunking.

use super::{lesson, strs, Drill, Lesson, Phase::Building};

/// The Building-phase lessons (2.1 – 2.6).
pub fn lessons() -> Vec<Lesson> {
    vec![
        lesson(
            "2.1",
            "Top 200 Words",
            "Type the 200 most common words on sight.",
            Building,
            Drill::Corpus {
                asset: "english_200.json".to_string(),
                words: 40,
            },
            30.0,
            95.0,
            60,
        ),
        lesson(
            "2.2",
            "Bigram & Trigram Fluency",
            "Chain frequent two- and three-letter clusters.",
            Building,
            Drill::Ngrams {
                items: strs(&[
                    "the", "and", "ing", "ion", "ent", "tio", "for", "her", "tha", "nth", "int",
                    "ere", "ate", "his", "con", "res", "ver", "all", "ons", "nce",
                ]),
                count: 40,
            },
            34.0,
            95.0,
            60,
        ),
        lesson(
            "2.3",
            "Sentence Flow",
            "Type real sentences without breaking rhythm.",
            Building,
            Drill::Corpus {
                asset: "quotes.json".to_string(),
                words: 35,
            },
            38.0,
            95.0,
            60,
        ),
        lesson(
            "2.4",
            "Code Mode: Intro",
            "Brackets, semicolons, and Rust syntax.",
            Building,
            Drill::Corpus {
                asset: "code_rust.json".to_string(),
                words: 30,
            },
            34.0,
            94.0,
            60,
        ),
        lesson(
            "2.5",
            "Rhythm Training",
            "Even, metronomic keystrokes — consistency over speed.",
            Building,
            Drill::Corpus {
                asset: "english_200.json".to_string(),
                words: 40,
            },
            40.0,
            96.0,
            60,
        ),
        lesson(
            "2.6",
            "Symbol Sprints",
            "Common contracted and punctuated words at speed.",
            Building,
            Drill::Symbols {
                items: strs(&[
                    "it's", "don't", "you're", "we'll", "can't", "won't", "they've", "isn't",
                    "let's", "that's", "i'm", "he'd",
                ]),
                count: 32,
            },
            40.0,
            95.0,
            60,
        ),
    ]
}
