use std::io::{self, Write};
use std::path::PathBuf;

use clap::Args;

use super::{Command, read_changelog};
use crate::error::Error;

#[derive(Debug, Args)]
pub struct LintOpts {
    #[arg(default_value = "CHANGELOG.md", help = "The changelog to lint")]
    pub path: PathBuf,
}

pub struct Lint;

impl Command for Lint {
    type Opts = LintOpts;

    fn run(&self, opts: Self::Opts) -> Result<(), Error> {
        let input = read_changelog(&opts.path)?;
        notable::parse(&input).map_err(|source| Error::invalid_changelog(&opts.path, source))?;

        writeln!(
            io::stdout().lock(),
            "`{}` is a valid changelog",
            opts.path.display()
        )
        .map_err(Error::Stdout)
    }
}
