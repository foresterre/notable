pub mod lint;
pub mod merge_conflicts;
pub mod reorder;

use std::fs;
use std::path::Path;

use crate::error::Error;

pub trait Command {
    type Opts;

    fn run(&self, opts: Self::Opts) -> Result<(), Error>;
}

fn read_changelog(path: &Path) -> Result<String, Error> {
    fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })
}

fn invalid_changelog(path: &Path) -> impl FnOnce(notable::ParseError) -> Error + '_ {
    move |source| Error::Parse {
        path: path.to_path_buf(),
        source,
    }
}
