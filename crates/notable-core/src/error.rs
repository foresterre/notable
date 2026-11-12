use std::fmt;

use crate::ChangeKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("expected the name of a kind of change, found an empty string")]
pub struct EmptyChangeKind;

#[derive(Debug, Clone, thiserror::Error)]
pub enum DateError {
    #[error("expected a date formatted as `YYYY-MM-DD`")]
    Format,
    #[error(transparent)]
    OutOfRange(jiff::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Block {
    Heading { level: u8 },
    Paragraph,
    List,
    Html,
    CodeBlock,
    BlockQuote,
    Rule,
    Other,
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Block::Heading { level } => write!(f, "a level {level} heading"),
            Block::Paragraph => f.write_str("a paragraph"),
            Block::List => f.write_str("a list"),
            Block::Html => f.write_str("an HTML block"),
            Block::CodeBlock => f.write_str("a code block"),
            Block::BlockQuote => f.write_str("a block quote"),
            Block::Rule => f.write_str("a horizontal rule"),
            Block::Other => f.write_str("other content"),
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ParseError {
    #[error(transparent)]
    Title(#[from] TitleError),
    #[error(transparent)]
    Release(#[from] ReleaseError),
    #[error(transparent)]
    Section(#[from] SectionError),
    #[error(transparent)]
    Content(#[from] ContentError),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TitleError {
    #[error("expected a title like `# Changelog`, found the end of the changelog")]
    Missing,
    #[error("expected a title like `# Changelog`, found {found} at line {line}")]
    Expected { found: Block, line: usize },
    #[error("expected a single title, found a second title at line {line}")]
    Duplicate { line: usize },
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ReleaseError {
    #[error(
        "expected a release heading like `[1.0.0] - 2024-01-31`, found `{heading}` at line {line}"
    )]
    InvalidHeading { heading: String, line: usize },
    #[error("expected a valid release date, found `{value}` at line {line}")]
    InvalidDate {
        value: String,
        line: usize,
        #[source]
        reason: DateError,
    },
    #[error("expected a single `Unreleased` section, found a second one at line {line}")]
    DuplicateUnreleased { line: usize },
    #[error(
        "expected the `Unreleased` section before all releases, found it after a release at line {line}"
    )]
    MisplacedUnreleased { line: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SectionError {
    #[error("expected a section name like `### Added`, found an empty heading at line {line}")]
    MissingName { line: usize },
    #[error("expected a single `{kind}` section per release, found a second one at line {line}")]
    Duplicate { kind: ChangeKind, line: usize },
    #[error("expected a release heading like `## [1.0.0]`, found section `{kind}` at line {line}")]
    OutsideRelease { kind: String, line: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContentError {
    #[error("expected a section heading like `### Added`, found a list at line {line}")]
    EntriesOutsideSection { line: usize },
    #[error("expected a list of changes, found {found} at line {line}")]
    Unexpected { found: Block, line: usize },
}
