//! Corpus loading and word selection.
//!
//! Corpora are JSON assets authored under `content/src/corpus/` and embedded
//! into the binary at compile time via [`rust_embed`]. At runtime no files are
//! read from disk (preserving the zero-runtime-dependency guarantee) *unless*
//! the `TYPEMASTER_CONTENT_DIR` environment variable is set, in which case
//! matching files in that directory override the embedded assets. This is a
//! developer convenience for live-editing corpora without recompiling; it is
//! never required for normal use. Loaded entries are shared as
//! `Arc<Vec<String>>` so a corpus is parsed once per session (Quality rule 7).

use std::path::Path;
use std::sync::Arc;

use rand::distributions::{Distribution, WeightedIndex};
use rand::seq::SliceRandom;
use rand::Rng;
use serde::Deserialize;

use crate::{Error, Result};

/// Environment variable that, when set, points at a directory whose JSON files
/// override the embedded corpora (developer hot-reload).
pub const CONTENT_DIR_ENV: &str = "TYPEMASTER_CONTENT_DIR";

/// Embedded corpus assets, compiled in from `content/src/corpus/`.
#[derive(rust_embed::RustEmbed)]
#[folder = "../content/src/corpus"]
struct CorpusAssets;

/// The shape of a corpus entry list: words vs. whole sentences/snippets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CorpusKind {
    /// Individual words to be joined with spaces.
    Words,
    /// Complete sentences or quotes used verbatim.
    Sentences,
    /// Source-code snippets used verbatim.
    Code,
}

/// A parsed corpus: a named, language-tagged list of entries.
#[derive(Debug, Clone, Deserialize)]
pub struct Corpus {
    /// Stable corpus identifier, e.g. `"english_200"`.
    pub name: String,
    /// BCP-47-ish language tag, e.g. `"en"` or `"sw"`.
    pub language: String,
    /// Whether entries are words, sentences, or code.
    pub kind: CorpusKind,
    /// The corpus entries, shared for cheap cloning.
    #[serde(default)]
    pub entries: Arc<Vec<String>>,
}

impl Corpus {
    /// Loads and parses a corpus by asset file name (e.g. `"english_200.json"`).
    ///
    /// If [`CONTENT_DIR_ENV`] is set and contains a matching file, that file is
    /// used; otherwise the embedded asset is used.
    ///
    /// Errors if the asset is missing ([`Error::CorpusNotFound`]), malformed
    /// ([`Error::CorpusParse`]), or empty ([`Error::CorpusEmpty`]).
    pub fn load(asset: &str) -> Result<Corpus> {
        let dir = std::env::var_os(CONTENT_DIR_ENV);
        Self::load_from(dir.as_deref().map(Path::new), asset)
    }

    /// Loads a corpus, preferring `dir` (on disk) and falling back to the
    /// embedded asset. Passing `None` loads the embedded asset directly. This is
    /// the testable core of [`Corpus::load`].
    pub fn load_from(dir: Option<&Path>, asset: &str) -> Result<Corpus> {
        let bytes: Vec<u8> = match dir.map(|d| std::fs::read(d.join(asset))) {
            Some(Ok(b)) => b,
            // No override dir, or the file is absent there: use the embedded copy.
            _ => CorpusAssets::get(asset)
                .ok_or_else(|| Error::CorpusNotFound(asset.to_string()))?
                .data
                .into_owned(),
        };
        let corpus: Corpus =
            serde_json::from_slice(&bytes).map_err(|source| Error::CorpusParse {
                name: asset.to_string(),
                source,
            })?;
        if corpus.entries.is_empty() {
            return Err(Error::CorpusEmpty(asset.to_string()));
        }
        Ok(corpus)
    }

    /// Number of entries in the corpus.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the corpus has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Samples `count` entries uniformly at random (with replacement).
    pub fn sample(&self, count: usize, rng: &mut impl Rng) -> Vec<String> {
        (0..count)
            .filter_map(|_| self.entries.choose(rng).cloned())
            .collect()
    }

    /// Samples `count` entries, biasing toward entries that contain any of the
    /// user's `weak_keys`. Each weak key an entry contains adds to its weight;
    /// every entry retains a base weight of 1 so all remain reachable.
    ///
    /// Falls back to uniform sampling when `weak_keys` is empty.
    pub fn sample_weighted(
        &self,
        count: usize,
        weak_keys: &[char],
        rng: &mut impl Rng,
    ) -> Vec<String> {
        if weak_keys.is_empty() {
            return self.sample(count, rng);
        }
        let weights: Vec<u32> = self
            .entries
            .iter()
            .map(|e| {
                let hits = weak_keys.iter().filter(|&&k| e.contains(k)).count() as u32;
                1 + hits * 3
            })
            .collect();
        let Ok(dist) = WeightedIndex::new(&weights) else {
            return self.sample(count, rng);
        };
        (0..count)
            .map(|_| self.entries[dist.sample(rng)].clone())
            .collect()
    }

    /// Builds a typing target of about `word_count` words using the thread RNG.
    ///
    /// Convenience wrapper over [`Corpus::build_text`] for callers that do not
    /// manage their own RNG (e.g. the binary).
    pub fn build_text_default(&self, word_count: usize) -> String {
        self.build_text(word_count, &mut rand::thread_rng())
    }

    /// Builds a single typing target string of about `word_count` words.
    ///
    /// For [`CorpusKind::Words`], entries are sampled and space-joined. For
    /// sentences/code, whole entries are concatenated until the approximate
    /// word budget is met.
    pub fn build_text(&self, word_count: usize, rng: &mut impl Rng) -> String {
        match self.kind {
            CorpusKind::Words => self.sample(word_count, rng).join(" "),
            CorpusKind::Sentences | CorpusKind::Code => {
                let mut out: Vec<String> = Vec::new();
                let mut words = 0usize;
                while words < word_count {
                    let Some(entry) = self.entries.choose(rng) else {
                        break;
                    };
                    words += entry.split_whitespace().count().max(1);
                    out.push(entry.clone());
                }
                out.join(" ")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn loads_embedded_english_200() {
        let c = Corpus::load("english_200.json").unwrap();
        assert_eq!(c.name, "english_200");
        assert_eq!(c.language, "en");
        assert_eq!(c.kind, CorpusKind::Words);
        assert!(c.len() >= 200, "expected >=200 words, got {}", c.len());
    }

    #[test]
    fn loads_embedded_quotes() {
        let c = Corpus::load("quotes.json").unwrap();
        assert_eq!(c.kind, CorpusKind::Sentences);
        assert!(!c.is_empty());
    }

    #[test]
    fn missing_corpus_errors() {
        assert!(matches!(
            Corpus::load("does_not_exist.json"),
            Err(Error::CorpusNotFound(_))
        ));
    }

    #[test]
    fn disk_override_takes_precedence_then_falls_back() {
        let dir = std::env::temp_dir().join(format!(
            "tm_corpus_override_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("english_200.json"),
            r#"{"name":"override","language":"en","kind":"words","entries":["zzz"]}"#,
        )
        .unwrap();

        // The on-disk file overrides the embedded asset.
        let overridden = Corpus::load_from(Some(&dir), "english_200.json").unwrap();
        assert_eq!(overridden.name, "override");
        assert_eq!(overridden.len(), 1);

        // An asset absent from the dir falls back to the embedded copy.
        let fallback = Corpus::load_from(Some(&dir), "quotes.json").unwrap();
        assert_eq!(fallback.name, "quotes");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sample_returns_requested_count() {
        let c = Corpus::load("english_200.json").unwrap();
        let mut rng = StdRng::seed_from_u64(42);
        let words = c.sample(10, &mut rng);
        assert_eq!(words.len(), 10);
    }

    #[test]
    fn weighted_sampling_favors_weak_keys() {
        let c = Corpus::load("english_200.json").unwrap();
        let mut rng = StdRng::seed_from_u64(7);
        // 'z' and 'q' are rare; words containing them should appear far more
        // often under weighted sampling than their base frequency would imply.
        let weak = ['z', 'q'];
        let sample = c.sample_weighted(200, &weak, &mut rng);
        let with_weak = sample
            .iter()
            .filter(|w| w.contains('z') || w.contains('q'))
            .count();
        assert!(with_weak > 0, "weighted sample never hit a weak-key word");
    }

    #[test]
    fn build_text_words_is_space_joined() {
        let c = Corpus::load("english_200.json").unwrap();
        let mut rng = StdRng::seed_from_u64(1);
        let text = c.build_text(5, &mut rng);
        assert_eq!(text.split(' ').count(), 5);
    }
}
