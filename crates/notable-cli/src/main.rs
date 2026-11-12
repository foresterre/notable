mod commands;
mod error;

use std::error::Error as _;
use std::io::{self, Write};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::commands::Command;
use crate::commands::lint::{Lint, LintOpts};
use crate::commands::merge_conflicts::{MergeConflicts, MergeConflictsOpts};
use crate::commands::reorder::{Reorder, ReorderOpts};
use crate::error::Error;

#[derive(Debug, Parser)]
#[command(name = "notable", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Subcommands,
}

#[derive(Debug, Subcommand)]
enum Subcommands {
    #[command(about = "Check whether a changelog follows the keep-a-changelog format")]
    Lint(LintOpts),
    #[command(about = "Re-order the change kind sections of every release")]
    Reorder(ReorderOpts),
    #[command(about = "Solve merge conflicts in a changelog")]
    MergeConflicts(MergeConflictsOpts),
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        Subcommands::Lint(opts) => Lint.run(opts),
        Subcommands::Reorder(opts) => Reorder.run(opts),
        Subcommands::MergeConflicts(opts) => MergeConflicts.run(opts),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            report(&error);
            ExitCode::FAILURE
        }
    }
}

fn report(error: &Error) {
    let mut stderr = io::stderr().lock();
    let _ = writeln!(stderr, "error: {error}");

    let mut source = error.source();
    while let Some(cause) = source {
        let _ = writeln!(stderr, "  caused by: {cause}");
        source = cause.source();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use notable::ChangeKind;
    use std::path::Path;

    #[test]
    fn cli_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn binary_is_called_notable() {
        assert_eq!(Cli::command().get_name(), "notable");
    }

    #[test]
    fn lint_defaults_to_changelog_md() {
        let cli = Cli::try_parse_from(["notable", "lint"]).unwrap();

        let Subcommands::Lint(opts) = cli.command else {
            panic!("expected the lint command");
        };
        assert_eq!(opts.path, Path::new("CHANGELOG.md"));
    }

    #[test]
    fn reorder_defaults() {
        let cli = Cli::try_parse_from(["notable", "reorder"]).unwrap();

        let Subcommands::Reorder(opts) = cli.command else {
            panic!("expected the reorder command");
        };
        assert_eq!(opts.path, Path::new("CHANGELOG.md"));
        assert_eq!(opts.order, ChangeKind::STANDARD);
        assert!(!opts.write);
    }

    #[test]
    fn reorder_with_custom_order() {
        let cli = Cli::try_parse_from([
            "notable",
            "reorder",
            "docs/CHANGELOG.md",
            "--order",
            "fixed,Performance,added",
            "--write",
        ])
        .unwrap();

        let Subcommands::Reorder(opts) = cli.command else {
            panic!("expected the reorder command");
        };
        assert_eq!(opts.path, Path::new("docs/CHANGELOG.md"));
        assert_eq!(
            opts.order,
            [
                ChangeKind::Fixed,
                ChangeKind::Custom("Performance".into()),
                ChangeKind::Added,
            ]
        );
        assert!(opts.write);
    }

    #[test]
    fn merge_conflicts_subcommand() {
        let cli = Cli::try_parse_from(["notable", "merge-conflicts"]).unwrap();

        assert!(matches!(cli.command, Subcommands::MergeConflicts(_)));
    }

    #[test]
    fn reorder_rejects_an_empty_kind() {
        assert!(Cli::try_parse_from(["notable", "reorder", "--order", "Added,,Fixed"]).is_err());
    }
}
