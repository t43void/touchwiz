//! Phase 4 — Elite (150–300 WPM): sub-60ms intervals, zero cognitive overhead.

use super::{lesson, strs, Drill, Lesson, Phase::Elite};

/// The Elite-phase lessons (4.1 – 4.4).
pub fn lessons() -> Vec<Lesson> {
    vec![
        lesson(
            "4.1",
            "Rolling Sequences",
            "Overlapping trigram rolls — pre-position your fingers.",
            Elite,
            Drill::Ngrams {
                items: strs(&[
                    "the", "and", "ing", "ion", "ent", "her", "tha", "tio", "ate", "for", "ter",
                    "est", "ers", "ome", "ould", "ight", "tion", "ment", "ness", "able",
                ]),
                count: 45,
            },
            150.0,
            96.0,
            120,
        ),
        lesson(
            "4.2",
            "High-Density Text",
            "Technical writing and code — maximum information rate.",
            Elite,
            Drill::Corpus {
                asset: "code_rust.json".to_string(),
                words: 70,
            },
            150.0,
            96.0,
            120,
        ),
        lesson(
            "4.3",
            "Sustained 3-Minute Test",
            "Hold peak speed with no visual aids.",
            Elite,
            Drill::Corpus {
                asset: "quotes.json".to_string(),
                words: 120,
            },
            160.0,
            97.0,
            180,
        ),
        lesson(
            "4.4",
            "Peak Performance",
            "Warm up, peak, and review — the full protocol.",
            Elite,
            Drill::Corpus {
                asset: "english_200.json".to_string(),
                words: 90,
            },
            170.0,
            97.0,
            180,
        ),
    ]
}
