//! TypeMaster engine — pure logic, no terminal or database I/O.
//!
//! This crate contains the typing-science core of TypeMaster: metric formulas
//! ([`metrics`]), the per-session state machine ([`session`]), corpus loading
//! ([`corpus`]), the adaptive SM-2 scheduler ([`adaptive`]), and per-key
//! heatmap aggregation ([`heatmap`]). It performs no terminal rendering and no
//! persistence; those live in the `typemaster` binary crate.

pub mod adaptive;
pub mod corpus;
pub mod curriculum;
pub mod heatmap;
pub mod import;
pub mod metrics;
pub mod session;

pub use metrics::Finger;
pub use session::{Keystroke, Session, SessionState};

/// All fallible operations in the engine return [`Result`] with this error type.
///
/// No `String` errors are used anywhere in the engine (Quality rule 8); every
/// failure mode is an explicit variant.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A corpus asset could not be found among the embedded resources.
    #[error("corpus asset not found: {0}")]
    CorpusNotFound(String),

    /// A corpus asset failed to parse as the expected JSON shape.
    #[error("failed to parse corpus `{name}`: {source}")]
    CorpusParse {
        /// The corpus asset name that failed to parse.
        name: String,
        /// The underlying serde error.
        #[source]
        source: serde_json::Error,
    },

    /// A corpus contained no usable entries.
    #[error("corpus `{0}` is empty")]
    CorpusEmpty(String),

    /// An operation was attempted that is invalid for the current session state.
    #[error("invalid session transition: {0}")]
    InvalidTransition(&'static str),
}

/// Convenience alias for engine results.
pub type Result<T> = std::result::Result<T, Error>;
