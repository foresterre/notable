use std::io;
use std::path::{Path, PathBuf};

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
    #[error("`{}` isn't a valid changelog", path.display())]
    Parse {
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
