//! The Output tree: the directory receiving everything one build produces (Kbuild's `O=`).

use std::path::{Component, Path, PathBuf};

use crate::source_tree::SourceTree;

#[derive(Debug)]
pub struct OutputTree {
    pub root: PathBuf,
}

impl OutputTree {
    /// The Output tree named by `-O <dir>`, relative to `cwd`. Rejects one that is, or contains,
    /// the Source tree: Makit never writes into the Source tree.
    pub fn new(dir: &Path, cwd: &Path, source_tree: &SourceTree) -> Result<Self, PathBuf> {
        let root = normalize(&cwd.join(dir));
        if normalize(&source_tree.root).starts_with(&root) {
            return Err(root);
        }
        Ok(Self { root })
    }
}

/// Resolves `.` and `..` lexically, without touching the filesystem (the Output tree may not exist
/// yet).
fn normalize(path: &Path) -> PathBuf {
    let mut normal = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normal.pop();
            }
            other => normal.push(other),
        }
    }
    normal
}
