//! Filesystem paths, resolved via XDG conventions (specification Section 7).

use std::path::PathBuf;

use directories::ProjectDirs;

/// The data directory, e.g. `$XDG_DATA_HOME/touchwiz` on Linux.
///
/// Returns `None` if no home directory can be determined.
pub fn data_dir() -> Option<PathBuf> {
    ProjectDirs::from("", "", "touchwiz").map(|p| p.data_dir().to_path_buf())
}

/// Full path to the SQLite database file, `<data_dir>/data.db`.
pub fn database_path() -> Option<PathBuf> {
    data_dir().map(|d| d.join("data.db"))
}
