//! The Output tree: the directory receiving everything one build produces (Kbuild's `O=`).

use std::fmt;
use std::path::{Component, Path, PathBuf};

use crate::source_tree::{MAKIT_DIR, SourceTree};

#[derive(Debug)]
pub struct OutputTree {
    pub root: PathBuf,
}

/// Where an Output tree must not go.
#[derive(Debug)]
pub enum OutputTreeError {
    /// It is the Source tree or one of its ancestors: Makit never writes into the Source tree.
    OverlapsSourceTree(PathBuf),
    /// It is inside `.makit/`, which holds only committed project files.
    InsideMakitDir(PathBuf),
}

impl fmt::Display for OutputTreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OverlapsSourceTree(dir) => write!(
                f,
                "the Output tree {} must not be or contain the Source tree; \
                 use a subdirectory (-O out) or a directory elsewhere",
                dir.display()
            ),
            Self::InsideMakitDir(dir) => write!(
                f,
                "the Output tree {} must not be inside {MAKIT_DIR}/",
                dir.display()
            ),
        }
    }
}

impl OutputTree {
    /// The Output tree named by `-O <dir>`, relative to `cwd`.
    pub fn new(dir: &Path, cwd: &Path, source_tree: &SourceTree) -> Result<Self, OutputTreeError> {
        let root = normalize(&cwd.join(dir));
        let real_root = resolve_symlinks(&root);
        let real_source = resolve_symlinks(&source_tree.root);
        if real_source.starts_with(&real_root) {
            return Err(OutputTreeError::OverlapsSourceTree(root));
        }
        if real_root.starts_with(real_source.join(MAKIT_DIR)) {
            return Err(OutputTreeError::InsideMakitDir(root));
        }
        Ok(Self { root })
    }
}

/// Resolves `.` and `..` lexically, without touching the filesystem.
pub fn normalize(path: &Path) -> PathBuf {
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

/// Resolves symlinks in the longest existing prefix of `path`; the Output tree may not exist yet.
fn resolve_symlinks(path: &Path) -> PathBuf {
    let mut missing = Vec::new();
    let mut existing = path;
    loop {
        if let Ok(real) = existing.canonicalize() {
            return missing
                .iter()
                .rev()
                .fold(real, |path, name| path.join(name));
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                missing.push(name.to_owned());
                existing = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
}
