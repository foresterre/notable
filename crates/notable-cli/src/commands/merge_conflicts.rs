use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use clap::Args;
use notable_merge::{Conflict, ResolveError, UnresolvableConflict};

use super::{Command, read_changelog};
use crate::error::Error;

mod interactive;

#[derive(Debug, Args)]
pub struct MergeConflictsOpts {
    #[arg(
        default_value = "CHANGELOG.md",
        help = "The changelog with merge conflicts"
    )]
    pub path: PathBuf,

    #[arg(
        long,
        help = "Write the resolved changelog back, instead of suggesting how to resolve it"
    )]
    pub resolve: bool,

    #[arg(
        long,
        conflicts_with = "resolve",
        help = "Print the suggestions as a unified diff, which can be applied with `git apply`"
    )]
    pub diff: bool,

    #[arg(
        long,
        short = 'i',
        conflicts_with_all = ["resolve", "diff"],
        help = "Ask for each conflict whether to apply its resolution"
    )]
    pub interactive: bool,
}

pub struct MergeConflicts;

impl Command for MergeConflicts {
    type Opts = MergeConflictsOpts;

    fn run(&self, opts: Self::Opts) -> Result<(), Error> {
        let input = read_changelog(&opts.path)?;
        let conflicts =
            notable_merge::find_conflicts(&input).map_err(|source| Error::Conflicts {
                path: opts.path.clone(),
                source,
            })?;

        if conflicts.is_empty() {
            return writeln!(
                io::stdout().lock(),
                "`{}` has no merge conflicts",
                opts.path.display()
            )
            .map_err(Error::Stdout);
        }

        if opts.interactive {
            return resolve_interactively(&opts.path, &input, &conflicts);
        }

        if opts.diff {
            diff(&opts.path, &input, &conflicts).map_err(Error::Stdout)?;
        } else if !opts.resolve {
            suggest(&opts.path, &conflicts).map_err(Error::Stdout)?;
        }

        let output = notable_merge::resolve_conflicts(&input)
            .map_err(|source| resolve_error(opts.path.clone(), source))?;

        if !opts.resolve {
            return Ok(());
        }

        fs::write(&opts.path, output).map_err(|source| Error::Write {
            path: opts.path.clone(),
            source,
        })?;

        writeln!(
            io::stdout().lock(),
            "resolved the merge conflicts in `{}`",
            opts.path.display()
        )
        .map_err(Error::Stdout)
    }
}

fn resolve_interactively(path: &Path, input: &str, conflicts: &[Conflict]) -> Result<(), Error> {
    note(&mut io::stdout().lock(), conflicts).map_err(Error::Stdout)?;

    let accepted = interactive::choose(
        path,
        input,
        conflicts,
        &mut io::stdin().lock(),
        &mut io::stdout().lock(),
    )?;
    let unresolved: Vec<UnresolvableConflict> = conflicts
        .iter()
        .filter_map(|conflict| conflict.outcome.clone().err())
        .collect();

    if !accepted.is_empty() {
        let output = notable_merge::apply_resolutions(input, accepted.iter().copied());
        if accepted.len() == conflicts.len() {
            notable::parse(&output).map_err(|source| Error::InvalidResolution {
                path: path.to_path_buf(),
                source,
            })?;
        }

        fs::write(path, output).map_err(|source| Error::Write {
            path: path.to_path_buf(),
            source,
        })?;
    }

    writeln!(
        io::stdout().lock(),
        "resolved {} of {} merge conflicts in `{}`",
        accepted.len(),
        conflicts.len(),
        path.display()
    )
    .map_err(Error::Stdout)?;

    if unresolved.is_empty() {
        return Ok(());
    }

    Err(Error::Unresolved {
        path: path.to_path_buf(),
        conflicts: unresolved,
    })
}

fn suggest(path: &Path, conflicts: &[Conflict]) -> io::Result<()> {
    let mut stdout = io::stdout().lock();

    for conflict in conflicts {
        let Ok(resolved) = &conflict.outcome else {
            continue;
        };

        let location = format!("{}:{}", path.display(), conflict.line);
        if resolved.trim().is_empty() {
            writeln!(stdout, "{location}: resolve the conflict by removing it\n")?;
            continue;
        }

        writeln!(stdout, "{location}: resolve the conflict with:\n")?;
        for line in resolved.lines() {
            match line.trim_end() {
                "" => writeln!(stdout)?,
                line => writeln!(stdout, "    {line}")?,
            }
        }
        writeln!(stdout)?;
    }

    note(&mut stdout, conflicts)
}

fn diff(path: &Path, input: &str, conflicts: &[Conflict]) -> io::Result<()> {
    let patch = notable_merge::unified_diff(&patch_path(path), input, conflicts);
    io::stdout().lock().write_all(patch.as_bytes())?;

    note(&mut io::stderr().lock(), conflicts)
}

fn note(out: &mut impl Write, conflicts: &[Conflict]) -> io::Result<()> {
    if conflicts.iter().any(|conflict| conflict.base.is_none()) {
        writeln!(
            out,
            "note: run `git config merge.conflictStyle zdiff3` before merging, so notable can tell edited entries apart from new ones"
        )?;
    }

    Ok(())
}

fn patch_path(path: &Path) -> String {
    path.components()
        .filter(|component| !matches!(component, Component::CurDir))
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn resolve_error(path: PathBuf, error: ResolveError) -> Error {
    match error {
        ResolveError::Markers(source) => Error::Conflicts { path, source },
        ResolveError::Unresolvable(conflicts) => Error::Unresolved { path, conflicts },
        ResolveError::Parse(source) => Error::InvalidResolution { path, source },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yare::parameterized;

    #[parameterized(
        file = { "CHANGELOG.md", "CHANGELOG.md" },
        nested = { "docs/CHANGELOG.md", "docs/CHANGELOG.md" },
        current_dir = { "./docs/./CHANGELOG.md", "docs/CHANGELOG.md" },
        parent_dir = { "../CHANGELOG.md", "../CHANGELOG.md" },
    )]
    fn patch_paths(path: &str, expected: &str) {
        assert_eq!(patch_path(Path::new(path)), expected);
    }
}
