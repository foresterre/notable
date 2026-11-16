use std::fmt;

use notable::{ChangeKind, ParseError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConflictMarker {
    Start,
    Base,
    Separator,
    End,
}

impl ConflictMarker {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConflictMarker::Start => "<<<<<<<",
            ConflictMarker::Base => "|||||||",
            ConflictMarker::Separator => "=======",
            ConflictMarker::End => ">>>>>>>",
        }
    }
}

impl fmt::Display for ConflictMarker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConflictMarkerError {
    #[error("expected a `{expected}` conflict marker, found `{found}` at line {line}")]
    Unexpected {
        expected: ConflictMarker,
        found: ConflictMarker,
        line: usize,
    },
    #[error(
        "expected a `>>>>>>>` conflict marker to close the conflict at line {line}, found the end of the changelog"
    )]
    Unterminated { line: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("can't resolve the conflict at line {line}: {reason}")]
pub struct UnresolvableConflict {
    pub line: usize,
    pub reason: UnresolvableReason,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UnresolvableReason {
    #[error("both sides changed the entry `{text}`")]
    EditedEntry { text: String },
    #[error("one side removed the `{kind}` section, while the other side changed it")]
    RemovedSection { kind: ChangeKind },
    #[error("both sides changed the link `[{label}]`")]
    ConflictingLink { label: String },
    #[error("expected entries, sections or links, found `{text}`")]
    Unrecognized { text: String },
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ResolveError {
    #[error(transparent)]
    Markers(#[from] ConflictMarkerError),
    #[error("expected conflicts which can be resolved, found {} which can't", .0.len())]
    Unresolvable(Vec<UnresolvableConflict>),
    #[error("expected a valid changelog after resolving the conflicts")]
    Parse(#[source] ParseError),
}
