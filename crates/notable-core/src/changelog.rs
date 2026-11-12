use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

use crate::{Date, EmptyChangeKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Changelog {
    pub title: String,
    pub description: String,
    pub unreleased: Option<Unreleased>,
    pub releases: Vec<Release>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unreleased {
    pub link: Option<String>,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    pub date: Option<Date>,
    pub yanked: bool,
    pub link: Option<String>,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub kind: ChangeKind,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChangeKind {
    Added,
    Changed,
    Deprecated,
    Removed,
    Fixed,
    Security,
    Custom(Cow<'static, str>),
}

impl ChangeKind {
    pub const STANDARD: [ChangeKind; 6] = [
        ChangeKind::Added,
        ChangeKind::Changed,
        ChangeKind::Deprecated,
        ChangeKind::Removed,
        ChangeKind::Fixed,
        ChangeKind::Security,
    ];

    pub fn as_str(&self) -> &str {
        match self {
            ChangeKind::Added => "Added",
            ChangeKind::Changed => "Changed",
            ChangeKind::Deprecated => "Deprecated",
            ChangeKind::Removed => "Removed",
            ChangeKind::Fixed => "Fixed",
            ChangeKind::Security => "Security",
            ChangeKind::Custom(name) => name,
        }
    }
}

impl fmt::Display for ChangeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ChangeKind {
    type Err = EmptyChangeKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let name = s.trim();
        if name.is_empty() {
            return Err(EmptyChangeKind);
        }

        let kind = ChangeKind::STANDARD
            .into_iter()
            .find(|kind| kind.as_str().eq_ignore_ascii_case(name))
            .unwrap_or_else(|| ChangeKind::Custom(Cow::Owned(name.to_string())));

        Ok(kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_kind_round_trips() {
        for kind in ChangeKind::STANDARD {
            assert_eq!(kind.to_string().parse::<ChangeKind>(), Ok(kind));
        }
    }

    #[test]
    fn change_kind_ignores_case_and_whitespace() {
        assert_eq!(" security ".parse::<ChangeKind>(), Ok(ChangeKind::Security));
        assert_eq!("FIXED".parse::<ChangeKind>(), Ok(ChangeKind::Fixed));
    }

    #[test]
    fn custom_change_kind() {
        let kind = " Performance ".parse::<ChangeKind>().unwrap();

        assert_eq!(kind, ChangeKind::Custom(Cow::Borrowed("Performance")));
        assert_eq!(kind.as_str(), "Performance");
        assert_eq!(kind.to_string(), "Performance");
    }

    #[test]
    fn empty_change_kind() {
        assert_eq!("".parse::<ChangeKind>(), Err(EmptyChangeKind));
        assert_eq!("  ".parse::<ChangeKind>(), Err(EmptyChangeKind));
    }
}
