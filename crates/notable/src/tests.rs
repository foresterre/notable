use super::*;

const KEEP_A_CHANGELOG: &str = r#"<!-- markdownlint-disable MD024 -->
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- v1.1 Brazilian Portuguese translation.
- v1.1 German Translation

### Changed

- Use frontmatter title & description in each language version template

## [1.1.1] - 2023-03-05

### Added

- Arabic translation (#444).
- v1.1 French translation.
  With a second line.

### Fixed

* Improve French translation (#377).
  - Including a nested list.

### Removed

1. Trademark sign previously shown after the project description.

## [0.0.1] - 2014-05-31 [YANKED]

### Security

- Initial release.

[unreleased]: https://github.com/olivierlacan/keep-a-changelog/compare/v1.1.1...HEAD
[1.1.1]: https://github.com/olivierlacan/keep-a-changelog/compare/v1.0.0...v1.1.1
[0.0.1]: https://github.com/olivierlacan/keep-a-changelog/releases/tag/v0.0.1
"#;

fn entries(texts: &[&str]) -> Vec<Entry> {
    texts
        .iter()
        .map(|text| Entry {
            text: text.to_string(),
        })
        .collect()
}

fn date(value: &str) -> Option<Date> {
    Some(value.parse().unwrap())
}

#[test]
fn parses_keep_a_changelog() {
    let changelog = parse(KEEP_A_CHANGELOG).unwrap();

    assert_eq!(changelog.title, "Changelog");
    assert!(
        changelog
            .description
            .starts_with("All notable changes to this project")
    );
    assert!(
        changelog
            .description
            .ends_with("(https://semver.org/spec/v2.0.0.html).")
    );

    assert_eq!(
        changelog.unreleased,
        Some(Unreleased {
            link: Some(
                "https://github.com/olivierlacan/keep-a-changelog/compare/v1.1.1...HEAD"
                    .to_string()
            ),
            sections: vec![
                Section {
                    kind: ChangeKind::Added,
                    entries: entries(&[
                        "v1.1 Brazilian Portuguese translation.",
                        "v1.1 German Translation",
                    ]),
                },
                Section {
                    kind: ChangeKind::Changed,
                    entries: entries(&[
                        "Use frontmatter title & description in each language version template",
                    ]),
                },
            ],
        })
    );

    assert_eq!(
        changelog.releases,
        vec![
            Release {
                version: "1.1.1".to_string(),
                date: date("2023-03-05"),
                yanked: false,
                link: Some(
                    "https://github.com/olivierlacan/keep-a-changelog/compare/v1.0.0...v1.1.1"
                        .to_string()
                ),
                sections: vec![
                    Section {
                        kind: ChangeKind::Added,
                        entries: entries(&[
                            "Arabic translation (#444).",
                            "v1.1 French translation.\n  With a second line.",
                        ]),
                    },
                    Section {
                        kind: ChangeKind::Fixed,
                        entries: entries(&[
                            "Improve French translation (#377).\n  - Including a nested list.",
                        ]),
                    },
                    Section {
                        kind: ChangeKind::Removed,
                        entries: entries(&[
                            "Trademark sign previously shown after the project description.",
                        ]),
                    },
                ],
            },
            Release {
                version: "0.0.1".to_string(),
                date: date("2014-05-31"),
                yanked: true,
                link: Some(
                    "https://github.com/olivierlacan/keep-a-changelog/releases/tag/v0.0.1"
                        .to_string()
                ),
                sections: vec![Section {
                    kind: ChangeKind::Security,
                    entries: entries(&["Initial release."]),
                }],
            },
        ]
    );
}

#[test]
fn release_headings_without_links() {
    let changelog = parse(
        "# Changelog\n\n## [Unreleased]\n\n## [1.0.0] - 2020-01-01\n\n## 0.9.0 - 2019-12-31\n\n## 0.1.0\n",
    )
    .unwrap();

    assert_eq!(changelog.description, "");
    assert_eq!(
        changelog.unreleased,
        Some(Unreleased {
            link: None,
            sections: vec![]
        })
    );

    let releases: Vec<_> = changelog
        .releases
        .iter()
        .map(|release| (release.version.as_str(), release.date, release.link.clone()))
        .collect();
    assert_eq!(
        releases,
        vec![
            ("1.0.0", date("2020-01-01"), None),
            ("0.9.0", date("2019-12-31"), None),
            ("0.1.0", None, None),
        ]
    );
}

#[test]
fn release_heading_with_inline_link() {
    let changelog =
        parse("# Changelog\n\n## [1.0.0](https://example.com/v1.0.0) – 2020-01-01\n").unwrap();

    assert_eq!(changelog.releases[0].version, "1.0.0");
    assert_eq!(
        changelog.releases[0].link.as_deref(),
        Some("https://example.com/v1.0.0")
    );
    assert_eq!(changelog.releases[0].date, date("2020-01-01"));
}

#[test]
fn yanked_without_date() {
    let changelog = parse("# Changelog\n\n## [1.0.0] [yanked]\n").unwrap();

    assert!(changelog.releases[0].yanked);
    assert_eq!(changelog.releases[0].date, None);
}

#[test]
fn description_without_releases() {
    let changelog = parse("# Changelog\n\nNothing yet.\n\n- a list is fine here\n").unwrap();

    assert_eq!(
        changelog.description,
        "Nothing yet.\n\n- a list is fine here"
    );
    assert_eq!(changelog.unreleased, None);
    assert!(changelog.releases.is_empty());
}

#[test]
fn html_comments_are_ignored_in_releases() {
    let changelog =
        parse("# Changelog\n\n## [1.0.0]\n\n<!-- nothing to see -->\n\n### Added\n\n- a\n")
            .unwrap();

    assert_eq!(changelog.releases[0].sections[0].entries, entries(&["a"]));
}

fn assert_errors<E: PartialEq + std::fmt::Debug + std::fmt::Display>(
    cases: &[(&str, E)],
    domain: fn(ParseError) -> Option<E>,
) {
    for (input, expected) in cases {
        let error = parse(input).unwrap_err();
        let message = error.to_string();
        let found = domain(error.clone());
        assert_eq!(
            found.as_ref(),
            Some(expected),
            "input:\n{input}\nerror: {error:?}"
        );
        assert_eq!(message, expected.to_string());
    }
}

#[test]
fn title_errors() {
    assert_errors(
        &[
            ("", TitleError::Missing),
            ("<!-- only a comment -->\n", TitleError::Missing),
            (
                "Some text\n\n# Changelog\n",
                TitleError::Expected {
                    found: Block::Paragraph,
                    line: 1,
                },
            ),
            (
                "## [1.0.0]\n",
                TitleError::Expected {
                    found: Block::Heading { level: 2 },
                    line: 1,
                },
            ),
            (
                "### Added\n",
                TitleError::Expected {
                    found: Block::Heading { level: 3 },
                    line: 1,
                },
            ),
            (
                "- a change\n",
                TitleError::Expected {
                    found: Block::List,
                    line: 1,
                },
            ),
            (
                "---\n",
                TitleError::Expected {
                    found: Block::Rule,
                    line: 1,
                },
            ),
            (
                "# Changelog\n\n# Another\n",
                TitleError::Duplicate { line: 3 },
            ),
        ],
        |error| match error {
            ParseError::Title(error) => Some(error),
            _ => None,
        },
    );
}

#[test]
fn section_errors() {
    assert_errors(
        &[
            (
                "# Changelog\n\n## [1.0.0]\n\n###\n",
                SectionError::MissingName { line: 5 },
            ),
            (
                "# Changelog\n\n## [1.0.0]\n\n### Docs\n\n### docs\n",
                SectionError::Duplicate {
                    kind: ChangeKind::Custom("docs".into()),
                    line: 7,
                },
            ),
            (
                "# Changelog\n\n## [1.0.0]\n\n### Added\n\n### Added\n",
                SectionError::Duplicate {
                    kind: ChangeKind::Added,
                    line: 7,
                },
            ),
            (
                "# Changelog\n\n### Added\n",
                SectionError::OutsideRelease {
                    kind: "Added".to_string(),
                    line: 3,
                },
            ),
        ],
        |error| match error {
            ParseError::Section(error) => Some(error),
            _ => None,
        },
    );
}

#[test]
fn content_errors() {
    assert_errors(
        &[
            (
                "# Changelog\n\n## [1.0.0]\n\n- a change\n",
                ContentError::EntriesOutsideSection { line: 5 },
            ),
            (
                "# Changelog\n\n## [1.0.0]\n\n### Added\n\nSome text.\n",
                ContentError::Unexpected {
                    found: Block::Paragraph,
                    line: 7,
                },
            ),
            (
                "# Changelog\n\n## [1.0.0]\n\n#### Details\n",
                ContentError::Unexpected {
                    found: Block::Heading { level: 4 },
                    line: 5,
                },
            ),
            (
                "# Changelog\n\n## [1.0.0]\n\n<details>hi</details>\n",
                ContentError::Unexpected {
                    found: Block::Html,
                    line: 5,
                },
            ),
            (
                "# Changelog\n\n## [1.0.0]\n\n### Added\n\n```\ncode\n```\n",
                ContentError::Unexpected {
                    found: Block::CodeBlock,
                    line: 7,
                },
            ),
            (
                "# Changelog\n\n## [1.0.0]\n\n### Added\n\n> quote\n",
                ContentError::Unexpected {
                    found: Block::BlockQuote,
                    line: 7,
                },
            ),
        ],
        |error| match error {
            ParseError::Content(error) => Some(error),
            _ => None,
        },
    );
}

fn release_error(input: &str) -> ReleaseError {
    match parse(input) {
        Err(ParseError::Release(error)) => error,
        other => panic!("expected a release error for:\n{input}\nfound: {other:?}"),
    }
}

#[test]
fn release_errors() {
    for (input, expected) in [
        ("# Changelog\n\n## - 2020-01-01\n", "- 2020-01-01"),
        (
            "# Changelog\n\n## [1.0.0] 2020-01-01\n",
            "[1.0.0] 2020-01-01",
        ),
        (
            "# Changelog\n\n## [Unreleased] - 2020-01-01\n",
            "[Unreleased] - 2020-01-01",
        ),
        (
            "# Changelog\n\n## [Unreleased] [YANKED]\n",
            "[Unreleased] [YANKED]",
        ),
    ] {
        assert!(
            matches!(
                release_error(input),
                ReleaseError::InvalidHeading { heading, line: 3 } if heading == expected
            ),
            "input:\n{input}"
        );
    }

    assert!(matches!(
        release_error("# Changelog\n\n## [Unreleased]\n\n## [Unreleased]\n"),
        ReleaseError::DuplicateUnreleased { line: 5 }
    ));
    assert!(matches!(
        release_error("# Changelog\n\n## [1.0.0]\n\n## [Unreleased]\n"),
        ReleaseError::MisplacedUnreleased { line: 5 }
    ));
}

#[test]
fn invalid_release_date() {
    let error = release_error("# Changelog\n\n## [1.0.0] - 2023-02-29\n");

    assert!(matches!(
        &error,
        ReleaseError::InvalidDate {
            value,
            line: 3,
            reason: DateError::OutOfRange(_),
        } if value == "2023-02-29"
    ));
    assert_eq!(
        error.to_string(),
        "expected a valid release date, found `2023-02-29` at line 3"
    );
    assert!(std::error::Error::source(&error).is_some());

    assert!(matches!(
        release_error("# Changelog\n\n## [1.0.0] - 2023-2-1\n"),
        ReleaseError::InvalidDate {
            reason: DateError::Format,
            ..
        }
    ));
}

#[test]
fn dates() {
    assert_eq!(
        parse_date("2023-03-05").unwrap(),
        Date::constant(2023, 3, 5)
    );
    assert_eq!(
        parse_date("2024-02-29").unwrap(),
        Date::constant(2024, 2, 29)
    );
    assert_eq!(
        parse_date("2000-02-29").unwrap(),
        Date::constant(2000, 2, 29)
    );

    for input in ["1900-02-29", "2023-13-01", "2023-04-31", "2023-04-00"] {
        assert!(
            matches!(parse_date(input), Err(DateError::OutOfRange(_))),
            "{input}"
        );
    }

    for input in [
        "",
        "2023",
        "2023-3-05",
        "23-03-05",
        "20230305",
        "2023-03-05-01",
        "2023-03-05T12:00",
        "2023/03/05",
        "+2023-03-05",
        "2023-+3-05",
        "２０２３-03-05",
    ] {
        assert!(
            matches!(parse_date(input), Err(DateError::Format)),
            "{input}"
        );
    }
}

#[test]
fn error_messages() {
    for (input, expected) in [
        (
            "",
            "expected a title like `# Changelog`, found the end of the changelog",
        ),
        (
            "## [1.0.0]\n",
            "expected a title like `# Changelog`, found a level 2 heading at line 1",
        ),
        (
            "# Changelog\n\n# Another\n",
            "expected a single title, found a second title at line 3",
        ),
        (
            "# Changelog\n\n## [1.0.0] 2020-01-01\n",
            "expected a release heading like `[1.0.0] - 2024-01-31`, found `[1.0.0] 2020-01-01` at line 3",
        ),
        (
            "# Changelog\n\n## [1.0.0] - 2020-1-1\n",
            "expected a valid release date, found `2020-1-1` at line 3",
        ),
        (
            "# Changelog\n\n## [Unreleased]\n\n## [Unreleased]\n",
            "expected a single `Unreleased` section, found a second one at line 5",
        ),
        (
            "# Changelog\n\n## [1.0.0]\n\n## [Unreleased]\n",
            "expected the `Unreleased` section before all releases, found it after a release at line 5",
        ),
        (
            "# Changelog\n\n## [1.0.0]\n\n###\n",
            "expected a section name like `### Added`, found an empty heading at line 5",
        ),
        (
            "# Changelog\n\n## [1.0.0]\n\n### Added\n\n### Added\n",
            "expected a single `Added` section per release, found a second one at line 7",
        ),
        (
            "# Changelog\n\n### Added\n",
            "expected a release heading like `## [1.0.0]`, found section `Added` at line 3",
        ),
        (
            "# Changelog\n\n## [1.0.0]\n\n- a change\n",
            "expected a section heading like `### Added`, found a list at line 5",
        ),
        (
            "# Changelog\n\n## [1.0.0]\n\n### Added\n\nSome text.\n",
            "expected a list of changes, found a paragraph at line 7",
        ),
    ] {
        assert_eq!(parse(input).unwrap_err().to_string(), expected);
    }
}

#[test]
fn custom_sections() {
    let changelog = parse(
        "# Changelog\n\n## [1.0.0]\n\n### Added\n\n- a\n\n### Performance\n\n- b\n\n### `Breaking` changes\n\n- c\n",
    )
    .unwrap();

    assert_eq!(
        changelog.releases[0].sections,
        vec![
            Section {
                kind: ChangeKind::Added,
                entries: entries(&["a"]),
            },
            Section {
                kind: ChangeKind::Custom("Performance".into()),
                entries: entries(&["b"]),
            },
            Section {
                kind: ChangeKind::Custom("Breaking changes".into()),
                entries: entries(&["c"]),
            },
        ]
    );
}

fn section_order(sections: &[Section]) -> Vec<(&str, Vec<&str>)> {
    sections
        .iter()
        .map(|section| {
            let entries = section
                .entries
                .iter()
                .map(|entry| entry.text.as_str())
                .collect();
            (section.kind.as_str(), entries)
        })
        .collect()
}

#[test]
fn reorder_sections() {
    let mut changelog = parse(
        "# Changelog

## [Unreleased]

### Fixed

- unreleased fix

### Performance

- faster

### Added

- unreleased feature

## [1.0.0]

### Docs

- docs

### Security

- patched

### Fixed

- fix

### Added

- feature
- another feature

## [0.1.0]
",
    )
    .unwrap();

    changelog.reorder_sections(&[
        ChangeKind::Added,
        ChangeKind::Custom("performance".into()),
        ChangeKind::Fixed,
        ChangeKind::Security,
    ]);

    assert_eq!(
        section_order(&changelog.unreleased.unwrap().sections),
        vec![
            ("Added", vec!["unreleased feature"]),
            ("Performance", vec!["faster"]),
            ("Fixed", vec!["unreleased fix"]),
        ]
    );
    assert_eq!(
        section_order(&changelog.releases[0].sections),
        vec![
            ("Added", vec!["feature", "another feature"]),
            ("Fixed", vec!["fix"]),
            ("Security", vec!["patched"]),
            ("Docs", vec!["docs"]),
        ]
    );
    assert!(changelog.releases[1].sections.is_empty());
}

#[test]
fn reorder_sections_keeps_unlisted_kinds_in_place() {
    let mut changelog =
        parse("# Changelog\n\n## [1.0.0]\n\n### Docs\n\n### Removed\n\n### Chores\n\n### Added\n")
            .unwrap();

    changelog.reorder_sections(&[ChangeKind::Added]);

    let kinds: Vec<_> = changelog.releases[0]
        .sections
        .iter()
        .map(|section| section.kind.as_str())
        .collect();
    assert_eq!(kinds, ["Added", "Docs", "Removed", "Chores"]);

    changelog.reorder_sections(&[]);
    let unchanged: Vec<_> = changelog.releases[0]
        .sections
        .iter()
        .map(|section| section.kind.as_str())
        .collect();
    assert_eq!(unchanged, ["Added", "Docs", "Removed", "Chores"]);
}
