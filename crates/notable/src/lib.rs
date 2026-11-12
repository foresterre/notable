//! ```
//! use notable::{ChangeKind, Date};
//!
//! let changelog = notable::parse(
//!     "# Changelog
//!
//! ### [Unreleased]
//!
//! #### Added
//!
//! - Support for yanked releases.
//!
//! ### [1.0.0] - 2024-01-31
//!
//! #### Fixed
//!
//! - Release dates are now validated.
//!
//! [1.0.0]: https://github.com/foresterre/notable/releases/tag/v1.0.0
//! ",
//! )?;
//!
//! assert_eq!(changelog.title, "Changelog");
//!
//! let unreleased = changelog.unreleased.unwrap();
//! assert_eq!(unreleased.sections[0].kind, ChangeKind::Added);
//! assert_eq!(unreleased.sections[0].entries[0].text, "Support for yanked releases.");
//!
//! let release = &changelog.releases[0];
//! assert_eq!(release.version, "1.0.0");
//! assert_eq!(release.date, Some(Date::constant(2024, 1, 31)));
//! assert_eq!(
//!     release.link.as_deref(),
//!     Some("https://github.com/foresterre/notable/releases/tag/v1.0.0"),
//! );
//! assert_eq!(release.sections[0].kind, ChangeKind::Fixed);
//! # Ok::<(), notable::ParseError>(())
//! ```

pub use notable_core::*;

use std::ops::Range;
use std::str::FromStr;

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

pub fn parse(input: &str) -> Result<Changelog, ParseError> {
    let mut builder = Builder::new(input);
    let mut events = Parser::new(input).into_offset_iter();

    while let Some((event, range)) = events.next() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let heading = read_heading(&mut events);
                builder.heading(level, heading, range)?;
            }
            Event::Start(Tag::List(_)) => {
                let items = read_list(&mut events);
                builder.list(items, range)?;
            }
            Event::Start(Tag::HtmlBlock) => {
                skip_block(&mut events);
                if !is_html_comment(&input[range.clone()]) {
                    builder.content(Block::Html, range)?;
                }
            }
            Event::Start(tag) => {
                skip_block(&mut events);
                builder.content(block(&tag), range)?;
            }
            Event::Rule => builder.content(Block::Rule, range)?,
            _ => builder.content(Block::Other, range)?,
        }
    }

    builder.finish()
}

struct Builder<'a> {
    input: &'a str,
    title: Option<String>,
    description_start: usize,
    description_end: Option<usize>,
    unreleased: Option<Unreleased>,
    releases: Vec<Release>,
}

impl<'a> Builder<'a> {
    fn new(input: &'a str) -> Self {
        Builder {
            input,
            title: None,
            description_start: 0,
            description_end: None,
            unreleased: None,
            releases: Vec::new(),
        }
    }

    fn line(&self, offset: usize) -> usize {
        self.input[..offset].matches('\n').count() + 1
    }

    fn require_title(&self, found: Block, line: usize) -> Result<(), TitleError> {
        match self.title {
            Some(_) => Ok(()),
            None => Err(TitleError::Expected { found, line }),
        }
    }

    fn in_release(&self) -> bool {
        self.unreleased.is_some() || !self.releases.is_empty()
    }

    fn sections_mut(&mut self) -> Option<&mut Vec<Section>> {
        match self.releases.last_mut() {
            Some(release) => Some(&mut release.sections),
            None => self
                .unreleased
                .as_mut()
                .map(|unreleased| &mut unreleased.sections),
        }
    }

    fn heading(
        &mut self,
        level: HeadingLevel,
        heading: Heading,
        range: Range<usize>,
    ) -> Result<(), ParseError> {
        let line = self.line(range.start);
        let found = Block::Heading { level: level as u8 };

        match level {
            HeadingLevel::H1 => {
                if self.title.is_some() {
                    return Err(TitleError::Duplicate { line }.into());
                }
                self.title = Some(heading.text.trim().to_string());
                self.description_start = range.end;
                Ok(())
            }
            HeadingLevel::H2 => {
                self.require_title(found, line)?;
                self.description_end.get_or_insert(range.start);
                Ok(self.release(heading, line)?)
            }
            HeadingLevel::H3 => {
                self.require_title(found, line)?;
                Ok(self.section(heading, line)?)
            }
            _ => self.content(found, range),
        }
    }

    fn release(&mut self, heading: Heading, line: usize) -> Result<(), ReleaseError> {
        let Heading { marked, link, .. } = heading;
        let invalid = || ReleaseError::InvalidHeading {
            heading: marked.trim().to_string(),
            line,
        };
        let parts = ReleaseHeading::split(&marked).ok_or_else(invalid)?;

        if parts.version.eq_ignore_ascii_case("unreleased") {
            if parts.date.is_some() || parts.yanked {
                return Err(invalid());
            }
            if self.unreleased.is_some() {
                return Err(ReleaseError::DuplicateUnreleased { line });
            }
            if !self.releases.is_empty() {
                return Err(ReleaseError::MisplacedUnreleased { line });
            }

            self.unreleased = Some(Unreleased {
                link,
                sections: Vec::new(),
            });
            return Ok(());
        }

        let date = parts
            .date
            .map(|value| {
                parse_date(value).map_err(|reason| ReleaseError::InvalidDate {
                    value: value.to_string(),
                    line,
                    reason,
                })
            })
            .transpose()?;

        self.releases.push(Release {
            version: parts.version.to_string(),
            date,
            yanked: parts.yanked,
            link,
            sections: Vec::new(),
        });
        Ok(())
    }

    fn section(&mut self, heading: Heading, line: usize) -> Result<(), SectionError> {
        let name = heading.text.trim();
        let Some(sections) = self.sections_mut() else {
            return Err(SectionError::OutsideRelease {
                kind: name.to_string(),
                line,
            });
        };

        let kind = name
            .parse::<ChangeKind>()
            .map_err(|UnknownChangeKind(name)| SectionError::UnknownKind { name, line })?;

        if sections.iter().any(|section| section.kind == kind) {
            return Err(SectionError::Duplicate { kind, line });
        }

        sections.push(Section {
            kind,
            entries: Vec::new(),
        });
        Ok(())
    }

    fn list(&mut self, items: Vec<Range<usize>>, range: Range<usize>) -> Result<(), ParseError> {
        let line = self.line(range.start);
        self.require_title(Block::List, line)?;

        let input = self.input;
        let Some(sections) = self.sections_mut() else {
            return Ok(());
        };
        let section = sections
            .last_mut()
            .ok_or(ContentError::EntriesOutsideSection { line })?;

        section.entries.extend(items.into_iter().map(|item| Entry {
            text: entry_text(&input[item]),
        }));
        Ok(())
    }

    fn content(&mut self, found: Block, range: Range<usize>) -> Result<(), ParseError> {
        let line = self.line(range.start);
        self.require_title(found, line)?;

        if self.in_release() {
            return Err(ContentError::Unexpected { found, line }.into());
        }
        Ok(())
    }

    fn finish(self) -> Result<Changelog, ParseError> {
        let title = self.title.ok_or(TitleError::Missing)?;
        let description_end = self.description_end.unwrap_or(self.input.len());

        Ok(Changelog {
            title,
            description: self.input[self.description_start..description_end]
                .trim()
                .to_string(),
            unreleased: self.unreleased,
            releases: self.releases,
        })
    }
}

#[derive(Default)]
struct Heading {
    text: String,
    marked: String,
    link: Option<String>,
}

fn read_heading<'a>(events: &mut impl Iterator<Item = (Event<'a>, Range<usize>)>) -> Heading {
    let mut heading = Heading::default();

    for (event, _) in events.by_ref() {
        match event {
            Event::End(TagEnd::Heading(_)) => break,
            Event::Text(text) | Event::Code(text) => {
                heading.text.push_str(&text);
                heading.marked.push_str(&text);
            }
            Event::SoftBreak | Event::HardBreak => {
                heading.text.push(' ');
                heading.marked.push(' ');
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                heading.marked.push('[');
                heading.link.get_or_insert_with(|| dest_url.into_string());
            }
            Event::End(TagEnd::Link) => heading.marked.push(']'),
            _ => {}
        }
    }

    heading
}

fn read_list<'a>(
    events: &mut impl Iterator<Item = (Event<'a>, Range<usize>)>,
) -> Vec<Range<usize>> {
    let mut items = Vec::new();
    let mut depth = 1usize;

    for (event, range) in events.by_ref() {
        match event {
            Event::Start(tag) => {
                depth += 1;
                if depth == 2 && matches!(tag, Tag::Item) {
                    items.push(range);
                }
            }
            Event::End(_) => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
    }

    items
}

fn skip_block<'a>(events: &mut impl Iterator<Item = (Event<'a>, Range<usize>)>) {
    let mut depth = 1usize;

    for (event, _) in events.by_ref() {
        match event {
            Event::Start(_) => depth += 1,
            Event::End(_) => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
    }
}

fn block(tag: &Tag<'_>) -> Block {
    match tag {
        Tag::Heading { level, .. } => Block::Heading {
            level: *level as u8,
        },
        Tag::Paragraph => Block::Paragraph,
        Tag::List(_) => Block::List,
        Tag::HtmlBlock => Block::Html,
        Tag::CodeBlock(_) => Block::CodeBlock,
        Tag::BlockQuote(_) => Block::BlockQuote,
        _ => Block::Other,
    }
}

fn is_html_comment(html: &str) -> bool {
    let html = html.trim();
    html.starts_with("<!--") && html.ends_with("-->")
}

fn entry_text(item: &str) -> String {
    let item = item.trim_start();
    let content = match item.strip_prefix(['-', '*', '+']) {
        Some(content) => content,
        None => {
            let digits = item.bytes().take_while(u8::is_ascii_digit).count();
            item[digits..].strip_prefix(['.', ')']).unwrap_or(item)
        }
    };

    content.trim().to_string()
}

struct ReleaseHeading<'a> {
    version: &'a str,
    date: Option<&'a str>,
    yanked: bool,
}

impl<'a> ReleaseHeading<'a> {
    fn split(heading: &'a str) -> Option<Self> {
        let heading = heading.trim();
        let (version, rest) = match heading.strip_prefix('[') {
            Some(heading) => heading.split_once(']')?,
            None => heading
                .split_once(char::is_whitespace)
                .unwrap_or((heading, "")),
        };

        let version = version.trim();
        if version.is_empty() {
            return None;
        }

        let rest = rest.trim();
        let (rest, yanked) = match strip_suffix_ignore_ascii_case(rest, "[yanked]") {
            Some(rest) => (rest.trim_end(), true),
            None => (rest, false),
        };

        let date = match rest {
            "" => None,
            rest => match rest.strip_prefix(['-', '–', '—'])?.trim() {
                "" => return None,
                date => Some(date),
            },
        };

        Some(ReleaseHeading {
            version,
            date,
            yanked,
        })
    }
}

fn parse_date(value: &str) -> Result<Date, DateError> {
    let mut parts = value.split('-');
    let (Some(year), Some(month), Some(day), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(DateError::Format);
    };

    Date::new(number(year, 4)?, number(month, 2)?, number(day, 2)?).map_err(DateError::OutOfRange)
}

fn number<T: FromStr>(digits: &str, len: usize) -> Result<T, DateError> {
    if digits.len() != len || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(DateError::Format);
    }

    digits.parse().map_err(|_| DateError::Format)
}

fn strip_suffix_ignore_ascii_case<'a>(text: &'a str, suffix: &str) -> Option<&'a str> {
    let start = text.len().checked_sub(suffix.len())?;
    let end = text.get(start..)?;

    end.eq_ignore_ascii_case(suffix).then(|| &text[..start])
}

#[cfg(test)]
mod tests;
