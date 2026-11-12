use clap::Args;

use super::Command;
use crate::error::Error;

#[derive(Debug, Args)]
pub struct MergeConflictsOpts {}

pub struct MergeConflicts;

impl Command for MergeConflicts {
    type Opts = MergeConflictsOpts;

    fn run(&self, _opts: Self::Opts) -> Result<(), Error> {
        todo!()
    }
}
