use std::io;
use std::path::{Path, PathBuf};

use notable_merge::UnresolvableConflict;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("couldn't read `{}`", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("couldn't write `{}`", path.display())]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("couldn't write to stdout")]
    Stdout(#[source] io::Error),
    #[error("couldn't read from stdin")]
    Stdin(#[source] io::Error),
    #[error("`{}` isn't a valid changelog", path.display())]
    Parse {
        path: PathBuf,
        #[source]
        source: notable::ParseError,
    },
    #[error("couldn't find the merge conflicts in `{}`", path.display())]
    Conflicts {
        path: PathBuf,
        #[source]
        source: notable_merge::ConflictMarkerError,
    },
    #[error("couldn't resolve the merge conflicts in `{}`{}", path.display(), list(conflicts))]
    Unresolved {
        path: PathBuf,
        conflicts: Vec<UnresolvableConflict>,
    },
    #[error("resolving the merge conflicts in `{}` doesn't result in a valid changelog", path.display())]
    InvalidResolution {
        path: PathBuf,
        #[source]
        source: notable::ParseError,
    },
}

impl Error {
    pub fn invalid_changelog(path: &Path, source: notable::ParseError) -> Self {
        Error::Parse {
            path: path.to_path_buf(),
            source,
        }
    }
}

fn list(conflicts: &[UnresolvableConflict]) -> String {
    conflicts
        .iter()
        .map(|conflict| format!("\n  - {conflict}"))
        .collect()
}
