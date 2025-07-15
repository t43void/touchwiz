//! Phase 3 — Speed Development (70–150 WPM): controlled accuracy → fluency.

use super::{lesson, Drill, Lesson, Phase::SpeedDevelopment};

/// The Speed-Development-phase lessons (3.1 – 3.6).
pub fn lessons() -> Vec<Lesson> {
    vec![
        lesson(
            "3.1",
            "Flow State",
            "Longer prose runs — settle into automatic typing.",
            SpeedDevelopment,
            Drill::Corpus {
                asset: "quotes.json".to_string(),
                words: 60,
            },
            70.0,
            95.0,
            120,
        ),
        lesson(
            "3.2",
            "Burst Training",
            "Short maximum-effort sprints to find your ceiling.",
            SpeedDevelopment,
            Drill::Corpus {
                asset: "english_200.json".to_string(),
                words: 25,
            },
            85.0,
            94.0,
            30,
        ),
        lesson(
            "3.3",
            "Word Frequency Mastery",
            "Drill the highest-value words until effortless.",
            SpeedDevelopment,
            Drill::Corpus {
                asset: "english_200.json".to_string(),
                words: 50,
            },
            90.0,
            95.0,
            60,
        ),
        lesson(
            "3.4",
            "Paragraph Mode",
            "Sustain accuracy across a full passage.",
            SpeedDevelopment,
            Drill::Corpus {
                asset: "quotes.json".to_string(),
                words: 80,
            },
            95.0,
            95.0,
            120,
        ),
        lesson(
            "3.5",
            "Code at Speed",
            "Type real code fluently, symbols and all.",
            SpeedDevelopment,
            Drill::Corpus {
                asset: "code_rust.json".to_string(),
                words: 50,
            },
            80.0,
            94.0,
            90,
        ),
        lesson(
            "3.6",
            "Numbers & Symbols",
            "Emails, URLs, paths, and digits inline.",
            SpeedDevelopment,
            Drill::TechTokens { count: 18 },
            70.0,
            93.0,
            90,
        ),
    ]
}
