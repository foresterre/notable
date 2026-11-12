use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use clap::Args;
use notable::ChangeKind;

use super::{Command, invalid_changelog, read_changelog};
use crate::error::Error;

#[derive(Debug, Args)]
pub struct ReorderOpts {
    #[arg(default_value = "CHANGELOG.md", help = "The changelog to re-order")]
    pub path: PathBuf,

    #[arg(
        long,
        value_name = "KINDS",
        value_delimiter = ',',
        default_values_t = ChangeKind::STANDARD,
        help = "The order of the sections, kinds which aren't listed go last"
    )]
    pub order: Vec<ChangeKind>,

    #[arg(
        long,
        help = "Write the result back to the changelog, instead of printing it"
    )]
    pub write: bool,
}

pub struct Reorder;

impl Command for Reorder {
    type Opts = ReorderOpts;

    fn run(&self, opts: Self::Opts) -> Result<(), Error> {
        let input = read_changelog(&opts.path)?;
        let output = notable::reorder_sections(&input, &opts.order)
            .map_err(invalid_changelog(&opts.path))?;

        if !opts.write {
            return io::stdout()
                .lock()
                .write_all(output.as_bytes())
                .map_err(Error::Stdout);
        }

        if output == input {
            return Ok(());
        }

        fs::write(&opts.path, output).map_err(|source| Error::Write {
            path: opts.path,
            source,
        })
    }
}
