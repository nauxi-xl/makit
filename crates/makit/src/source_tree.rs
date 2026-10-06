//! Finding the Source tree: the nearest directory, upwards from where makit runs, that holds a
//! Project manifest.

use std::path::{Path, PathBuf};

/// Location of the Project manifest relative to the Source tree root.
pub const MANIFEST_PATH: &str = ".makit/config.toml";

#[derive(Debug)]
pub struct SourceTree {
    pub root: PathBuf,
}

impl SourceTree {
    /// Searches `start` and its ancestors for a Project manifest.
    pub fn discover(start: &Path) -> Option<Self> {
        start
            .ancestors()
            .find(|dir| dir.join(MANIFEST_PATH).is_file())
            .map(|root| Self {
                root: root.to_path_buf(),
            })
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.root.join(MANIFEST_PATH)
    }
}
