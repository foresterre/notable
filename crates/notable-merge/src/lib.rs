use std::ops::Range;

use notable::{ChangeKind, EmptyChangeKind, parse};

pub use diff::{unified_diff, unified_diff_hunks};
pub use error::{
    ConflictMarker, ConflictMarkerError, ResolveError, UnresolvableConflict, UnresolvableReason,
};

mod diff;
mod error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    pub line: usize,
    pub end_line: usize,
    pub ours: String,
    pub base: Option<String>,
    pub theirs: String,
    pub outcome: Result<String, UnresolvableConflict>,
    range: Range<usize>,
}

pub fn find_conflicts(input: &str) -> Result<Vec<Conflict>, ConflictMarkerError> {
    let mut conflicts = Vec::new();
    let mut state = State::Outside;

    for line in lines(input) {
        let Some(found) = marker(line.text) else {
            continue;
        };

        state = match (state, found) {
            (State::Outside, ConflictMarker::Start) => State::Ours {
                open: Open {
                    line: line.number,
                    start: line.range.start,
                    eol: line.eol,
                },
                ours: line.range.end,
            },
            (State::Outside, ConflictMarker::Separator) => State::Outside,
            (State::Ours { open, ours }, ConflictMarker::Base) => State::Base {
                open,
                ours: ours..line.range.start,
                base: line.range.end,
            },
            (State::Ours { open, ours }, ConflictMarker::Separator) => State::Theirs {
                open,
                ours: ours..line.range.start,
                base: None,
                theirs: line.range.end,
            },
            (State::Base { open, ours, base }, ConflictMarker::Separator) => State::Theirs {
                open,
                ours,
                base: Some(base..line.range.start),
                theirs: line.range.end,
            },
            (
                State::Theirs {
                    open,
                    ours,
                    base,
                    theirs,
                },
                ConflictMarker::End,
            ) => {
                conflicts.push(conflict(
                    input,
                    Hunk {
                        open,
                        ours,
                        base,
                        theirs: theirs..line.range.start,
                        end: line.range.end,
                        end_line: line.number,
                        final_eol: !line.eol.is_empty(),
                    },
                ));
                State::Outside
            }
            (state, found) => {
                return Err(ConflictMarkerError::Unexpected {
                    expected: state.expected(),
                    found,
                    line: line.number,
                });
            }
        };
    }

    match state.open() {
        Some(open) => Err(ConflictMarkerError::Unterminated { line: open.line }),
        None => Ok(conflicts),
    }
}

pub fn resolve_conflicts(input: &str) -> Result<String, ResolveError> {
    let conflicts = find_conflicts(input)?;

    let unresolved: Vec<UnresolvableConflict> = conflicts
        .iter()
        .filter_map(|conflict| conflict.outcome.clone().err())
        .collect();
    if !unresolved.is_empty() {
        return Err(ResolveError::Unresolvable(unresolved));
    }

    let output = apply_resolutions(input, &conflicts);
    parse(&output).map_err(ResolveError::Parse)?;
    Ok(output)
}

pub fn apply_resolutions<'c>(
    input: &str,
    conflicts: impl IntoIterator<Item = &'c Conflict>,
) -> String {
    let mut resolved: Vec<(&Range<usize>, &str)> = conflicts
        .into_iter()
        .filter_map(|conflict| {
            let text = conflict.outcome.as_deref().ok()?;
            Some((&conflict.range, text))
        })
        .collect();
    resolved.sort_by_key(|(range, _)| range.start);
    resolved.dedup_by_key(|(range, _)| range.start);

    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;

    for (range, text) in resolved {
        output.push_str(&input[cursor..range.start]);
        output.push_str(text);
        cursor = range.end;
    }
    output.push_str(&input[cursor..]);

    output
}

struct Line<'a> {
    number: usize,
    text: &'a str,
    eol: &'a str,
    range: Range<usize>,
}

fn lines(input: &str) -> impl Iterator<Item = Line<'_>> {
    input
        .split_inclusive('\n')
        .scan(0, |offset, raw| {
            let start = *offset;
            *offset += raw.len();
            Some((start, raw))
        })
        .enumerate()
        .map(|(index, (start, raw))| {
            let text = strip_eol(raw);
            Line {
                number: index + 1,
                text,
                eol: &raw[text.len()..],
                range: start..start + raw.len(),
            }
        })
}

fn strip_eol(raw: &str) -> &str {
    match raw.strip_suffix('\n') {
        Some(line) => line.strip_suffix('\r').unwrap_or(line),
        None => raw,
    }
}

fn text_lines(text: &str) -> Vec<&str> {
    text.split_inclusive('\n').map(strip_eol).collect()
}

fn marker(line: &str) -> Option<ConflictMarker> {
    let (prefix, rest) = line.split_at_checked(7)?;
    let marker = match prefix {
        "<<<<<<<" => ConflictMarker::Start,
        "|||||||" => ConflictMarker::Base,
        "=======" => ConflictMarker::Separator,
        ">>>>>>>" => ConflictMarker::End,
        _ => return None,
    };

    let labelled = rest.starts_with(' ') && marker != ConflictMarker::Separator;
    (rest.is_empty() || labelled).then_some(marker)
}

struct Open<'a> {
    line: usize,
    start: usize,
    eol: &'a str,
}

enum State<'a> {
    Outside,
    Ours {
        open: Open<'a>,
        ours: usize,
    },
    Base {
        open: Open<'a>,
        ours: Range<usize>,
        base: usize,
    },
    Theirs {
        open: Open<'a>,
        ours: Range<usize>,
        base: Option<Range<usize>>,
        theirs: usize,
    },
}

impl<'a> State<'a> {
    fn expected(&self) -> ConflictMarker {
        match self {
            State::Outside => ConflictMarker::Start,
            State::Ours { .. } | State::Base { .. } => ConflictMarker::Separator,
            State::Theirs { .. } => ConflictMarker::End,
        }
    }

    fn open(self) -> Option<Open<'a>> {
        match self {
            State::Outside => None,
            State::Ours { open, .. } | State::Base { open, .. } | State::Theirs { open, .. } => {
                Some(open)
            }
        }
    }
}

struct Hunk<'a> {
    open: Open<'a>,
    ours: Range<usize>,
    base: Option<Range<usize>>,
    theirs: Range<usize>,
    end: usize,
    end_line: usize,
    final_eol: bool,
}

fn conflict(input: &str, hunk: Hunk<'_>) -> Conflict {
    let ours = &input[hunk.ours];
    let base = hunk.base.map(|base| &input[base]);
    let theirs = &input[hunk.theirs];
    let eol = match hunk.open.eol {
        "\r\n" => "\r\n",
        _ => "\n",
    };

    let outcome = resolve(ours, base, theirs)
        .map(|lines| join(&lines, eol, hunk.final_eol))
        .map_err(|reason| UnresolvableConflict {
            line: hunk.open.line,
            reason,
        });

    Conflict {
        line: hunk.open.line,
        end_line: hunk.end_line,
        ours: ours.to_string(),
        base: base.map(str::to_string),
        theirs: theirs.to_string(),
        outcome,
        range: hunk.open.start..hunk.end,
    }
}

fn join(lines: &[&str], eol: &str, final_eol: bool) -> String {
    let mut output = String::new();

    for (index, line) in lines.iter().enumerate() {
        output.push_str(line);
        if final_eol || index + 1 < lines.len() {
            output.push_str(eol);
        }
    }

    output
}

fn resolve<'a>(
    ours: &'a str,
    base: Option<&'a str>,
    theirs: &'a str,
) -> Result<Vec<&'a str>, UnresolvableReason> {
    let ours = text_lines(ours);
    let theirs = text_lines(theirs);
    let base = base.map(text_lines);

    if ours == theirs {
        return Ok(ours);
    }
    if let Some(base) = &base {
        if ours == *base {
            return Ok(theirs);
        }
        if theirs == *base {
            return Ok(ours);
        }
    }

    let ours = Side::parse(&ours)?;
    let theirs = Side::parse(&theirs)?;
    let base = match base {
        Some(base) => Side::parse(&base)?,
        None => Side::default(),
    };

    let merged = Side {
        leading: 0,
        trailing: 0,
        loose: merge_entries(&ours.loose, &base.loose, &theirs.loose)?,
        sections: merge_sections(&ours.sections, &base.sections, &theirs.sections)?,
        links: merge_links(&ours.links, &base.links, &theirs.links)?,
    };

    Ok(merged.render(&ours, &theirs))
}

#[derive(Default)]
struct Side<'a> {
    leading: usize,
    trailing: usize,
    loose: Vec<Entry<'a>>,
    sections: Vec<Section<'a>>,
    links: Vec<Link<'a>>,
}

#[derive(Clone)]
struct Entry<'a> {
    lines: Vec<&'a str>,
}

struct Section<'a> {
    heading: &'a str,
    kind: ChangeKind,
    entries: Vec<Entry<'a>>,
}

#[derive(Clone, Copy)]
struct Link<'a> {
    line: &'a str,
    label: &'a str,
    url: &'a str,
}

#[derive(PartialEq, Eq)]
enum BlockKind {
    Loose,
    Section(String),
    Links,
}

impl<'a> Side<'a> {
    fn parse(lines: &[&'a str]) -> Result<Self, UnresolvableReason> {
        let leading = lines.iter().take_while(|line| is_blank(line)).count();
        if leading == lines.len() {
            return Ok(Side {
                leading,
                ..Side::default()
            });
        }

        let trailing = lines.iter().rev().take_while(|line| is_blank(line)).count();
        let body = &lines[leading..lines.len() - trailing];
        let mut side = Side {
            leading,
            trailing,
            ..Side::default()
        };
        let mut entry: Option<Entry<'a>> = None;

        for (index, &line) in body.iter().enumerate() {
            if is_blank(line) {
                match entry.as_mut() {
                    Some(current) if continues_after_blank(&body[index + 1..]) => {
                        current.lines.push(line);
                    }
                    _ => side.push_entry(entry.take()),
                }
                continue;
            }

            if let Some(name) = section_heading(line) {
                if !side.links.is_empty() {
                    return Err(unrecognized(line));
                }
                side.push_entry(entry.take());
                let kind = name
                    .parse::<ChangeKind>()
                    .map_err(|EmptyChangeKind| unrecognized(line))?;
                side.sections.push(Section {
                    heading: line,
                    kind,
                    entries: Vec::new(),
                });
            } else if line.starts_with('#') {
                return Err(unrecognized(line));
            } else if let Some(link) = link(line) {
                side.push_entry(entry.take());
                side.links.push(link);
            } else if is_entry_start(line) {
                if !side.links.is_empty() {
                    return Err(unrecognized(line));
                }
                side.push_entry(entry.take());
                entry = Some(Entry { lines: vec![line] });
            } else if let Some(current) = entry.as_mut() {
                current.lines.push(line);
            } else {
                return Err(unrecognized(line));
            }
        }

        side.push_entry(entry);
        Ok(side)
    }

    fn push_entry(&mut self, entry: Option<Entry<'a>>) {
        let Some(entry) = entry else {
            return;
        };

        match self.sections.last_mut() {
            Some(section) => section.entries.push(entry),
            None => self.loose.push(entry),
        }
    }

    fn blocks(&self) -> Vec<BlockKind> {
        let loose = (!self.loose.is_empty()).then_some(BlockKind::Loose);
        let sections = self
            .sections
            .iter()
            .map(|section| BlockKind::Section(section.kind.as_str().to_ascii_lowercase()));
        let links = (!self.links.is_empty()).then_some(BlockKind::Links);

        loose.into_iter().chain(sections).chain(links).collect()
    }

    fn render(&self, ours: &Side<'_>, theirs: &Side<'_>) -> Vec<&'a str> {
        let blocks = self.blocks();
        let (Some(first), Some(last)) = (blocks.first(), blocks.last()) else {
            let blanks = [ours, theirs]
                .iter()
                .map(|side| side.leading + side.trailing)
                .max()
                .unwrap_or(0);
            return vec![""; blanks];
        };

        let leading = [ours, theirs]
            .iter()
            .filter(|side| side.blocks().first() == Some(first))
            .map(|side| side.leading)
            .max()
            .unwrap_or(0);
        let trailing = [ours, theirs]
            .iter()
            .filter(|side| side.blocks().last() == Some(last))
            .map(|side| side.trailing)
            .max()
            .unwrap_or(0);

        let loose = (!self.loose.is_empty()).then(|| entry_lines(&self.loose));
        let sections = self.sections.iter().map(|section| {
            let mut lines = vec![section.heading];
            if !section.entries.is_empty() {
                lines.push("");
                lines.extend(entry_lines(&section.entries));
            }
            lines
        });
        let links = (!self.links.is_empty())
            .then(|| self.links.iter().map(|link| link.line).collect::<Vec<_>>());

        let mut lines = vec![""; leading];
        for (index, block) in loose.into_iter().chain(sections).chain(links).enumerate() {
            if index > 0 {
                lines.push("");
            }
            lines.extend(block);
        }
        lines.extend(std::iter::repeat_n("", trailing));
        lines
    }
}

fn entry_lines<'a>(entries: &[Entry<'a>]) -> Vec<&'a str> {
    entries
        .iter()
        .flat_map(|entry| entry.lines.iter().copied())
        .collect()
}

impl Entry<'_> {
    fn key(&self) -> String {
        let mut lines = self.lines.iter().map(|line| line.trim());
        let first = lines.next().map(strip_marker).unwrap_or_default();

        std::iter::once(first)
            .chain(lines)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn summary(&self) -> String {
        self.lines
            .first()
            .map(|line| strip_marker(line.trim()))
            .unwrap_or_default()
            .to_string()
    }
}

fn is_blank(line: &str) -> bool {
    line.trim().is_empty()
}

fn continues_after_blank(rest: &[&str]) -> bool {
    rest.iter()
        .find(|line| !is_blank(line))
        .is_some_and(|line| line.starts_with([' ', '\t']))
}

fn section_heading(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("###")?;
    (rest.is_empty() || rest.starts_with([' ', '\t'])).then_some(rest)
}

fn link(line: &str) -> Option<Link<'_>> {
    let (label, rest) = line.strip_prefix('[')?.split_once("]:")?;
    if label.trim().is_empty() || label.contains(['[', ']']) {
        return None;
    }

    let url = rest.split_whitespace().next()?;
    Some(Link { line, label, url })
}

fn is_entry_start(line: &str) -> bool {
    strip_marker(line).len() != line.len()
}

fn strip_marker(line: &str) -> &str {
    if let Some(rest) = ["- ", "* ", "+ "]
        .iter()
        .find_map(|marker| line.strip_prefix(marker))
    {
        return rest;
    }

    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return line;
    }

    let rest = &line[digits..];
    rest.strip_prefix(". ")
        .or_else(|| rest.strip_prefix(") "))
        .unwrap_or(line)
}

fn unrecognized(line: &str) -> UnresolvableReason {
    UnresolvableReason::Unrecognized {
        text: line.trim().to_string(),
    }
}

fn contains(entries: &[Entry<'_>], key: &str) -> bool {
    entries.iter().any(|entry| entry.key() == key)
}

fn merge_entries<'a>(
    ours: &[Entry<'a>],
    base: &[Entry<'a>],
    theirs: &[Entry<'a>],
) -> Result<Vec<Entry<'a>>, UnresolvableReason> {
    let added_by_ours = ours.iter().any(|entry| !contains(base, &entry.key()));
    let added_by_theirs = theirs.iter().any(|entry| !contains(base, &entry.key()));
    let removed_by_both = base.iter().find(|entry| {
        let key = entry.key();
        !contains(ours, &key) && !contains(theirs, &key)
    });

    if let Some(entry) = removed_by_both {
        if added_by_ours && added_by_theirs {
            return Err(UnresolvableReason::EditedEntry {
                text: entry.summary(),
            });
        }
    }

    let kept_by_ours = ours.iter().filter(|entry| {
        let key = entry.key();
        !contains(base, &key) || contains(theirs, &key)
    });
    let added_by_theirs = theirs.iter().filter(|entry| {
        let key = entry.key();
        !contains(ours, &key) && !contains(base, &key)
    });

    Ok(kept_by_ours.chain(added_by_theirs).cloned().collect())
}

fn same_kind(left: &ChangeKind, right: &ChangeKind) -> bool {
    left.as_str().eq_ignore_ascii_case(right.as_str())
}

fn find_section<'s, 'a>(sections: &'s [Section<'a>], kind: &ChangeKind) -> Option<&'s Section<'a>> {
    sections
        .iter()
        .find(|section| same_kind(&section.kind, kind))
}

fn merge_sections<'a>(
    ours: &[Section<'a>],
    base: &[Section<'a>],
    theirs: &[Section<'a>],
) -> Result<Vec<Section<'a>>, UnresolvableReason> {
    let mut kinds: Vec<&ChangeKind> = Vec::new();
    for section in ours.iter().chain(theirs) {
        if !kinds.iter().any(|kind| same_kind(kind, &section.kind)) {
            kinds.push(&section.kind);
        }
    }

    let mut merged = Vec::new();
    for kind in kinds {
        let in_ours = find_section(ours, kind);
        let in_base = find_section(base, kind);
        let in_theirs = find_section(theirs, kind);

        let entries = merge_entries(
            in_ours.map_or(&[], |section| &section.entries),
            in_base.map_or(&[], |section| &section.entries),
            in_theirs.map_or(&[], |section| &section.entries),
        )?;

        let present = if in_ours.is_some() == in_base.is_some() {
            in_theirs.is_some()
        } else {
            in_ours.is_some()
        };
        if !present {
            if entries.is_empty() {
                continue;
            }
            return Err(UnresolvableReason::RemovedSection { kind: kind.clone() });
        }

        let Some(section) = in_ours.or(in_theirs) else {
            continue;
        };
        merged.push(Section {
            heading: section.heading,
            kind: section.kind.clone(),
            entries,
        });
    }

    Ok(merged)
}

fn same_label(left: &str, right: &str) -> bool {
    left.trim().eq_ignore_ascii_case(right.trim())
}

fn find_link<'a>(links: &[Link<'a>], label: &str) -> Option<Link<'a>> {
    links
        .iter()
        .find(|link| same_label(link.label, label))
        .copied()
}

fn merge_links<'a>(
    ours: &[Link<'a>],
    base: &[Link<'a>],
    theirs: &[Link<'a>],
) -> Result<Vec<Link<'a>>, UnresolvableReason> {
    let mut labels: Vec<&str> = Vec::new();
    for link in ours.iter().chain(theirs) {
        if !labels.iter().any(|label| same_label(label, link.label)) {
            labels.push(link.label);
        }
    }

    let mut merged = Vec::new();
    for label in labels {
        let in_ours = find_link(ours, label);
        let in_base = find_link(base, label);
        let in_theirs = find_link(theirs, label);
        let url = |link: Option<Link<'a>>| link.map(|link| link.url);

        let chosen = if url(in_ours) == url(in_base) {
            in_theirs
        } else if url(in_theirs) == url(in_base) || url(in_ours) == url(in_theirs) {
            in_ours
        } else {
            return Err(UnresolvableReason::ConflictingLink {
                label: label.trim().to_string(),
            });
        };

        merged.extend(chosen);
    }

    Ok(merged)
}

#[cfg(test)]
mod tests;
