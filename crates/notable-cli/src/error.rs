use std::io;
use std::path::PathBuf;

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
