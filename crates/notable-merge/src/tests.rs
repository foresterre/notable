use super::*;
use notable::ParseError;

use yare::parameterized;

fn outcome(input: &str) -> Result<String, UnresolvableReason> {
    let mut conflicts = find_conflicts(input).unwrap();
    assert_eq!(conflicts.len(), 1, "expected a single conflict");

    conflicts
        .remove(0)
        .outcome
        .map_err(|unresolvable| unresolvable.reason)
}

#[test]
fn finds_a_conflict() {
    let input = "# Changelog\n\n<<<<<<< HEAD\n- a\n=======\n- b\n>>>>>>> feature\n";
    let conflicts = find_conflicts(input).unwrap();

    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].line, 3);
    assert_eq!(conflicts[0].ours, "- a\n");
    assert_eq!(conflicts[0].base, None);
    assert_eq!(conflicts[0].theirs, "- b\n");
    assert_eq!(conflicts[0].outcome, Ok("- a\n- b\n".to_string()));
}

#[test]
fn finds_a_conflict_with_a_base() {
    let input = "<<<<<<< HEAD\n- a\n||||||| base\n- x\n=======\n- b\n>>>>>>> feature\n";
    let conflicts = find_conflicts(input).unwrap();

    assert_eq!(conflicts[0].base.as_deref(), Some("- x\n"));
}

#[test]
fn finds_several_conflicts() {
    let input = "<<<<<<< HEAD\n- a\n=======\n- b\n>>>>>>> feature\n\n<<<<<<< HEAD\n- c\n=======\n- d\n>>>>>>> feature\n";
    let lines: Vec<usize> = find_conflicts(input)
        .unwrap()
        .iter()
        .map(|conflict| conflict.line)
        .collect();

    assert_eq!(lines, [1, 7]);
}

#[parameterized(
    no_conflicts = { "# Changelog\n\n- a\n" },
    setext_heading = { "Changelog\n=======\n" },
    longer_markers = { "<<<<<<<< a\n>>>>>>>> b\n" },
    unlabelled_separator_only = { "=======\n" },
)]
fn no_conflicts(input: &str) {
    assert_eq!(find_conflicts(input).unwrap(), []);
}

#[parameterized(
    stray_end = { ">>>>>>> feature\n", ConflictMarker::Start, ConflictMarker::End, 1 },
    stray_base = { "||||||| base\n", ConflictMarker::Start, ConflictMarker::Base, 1 },
    nested = { "<<<<<<< HEAD\n<<<<<<< HEAD\n", ConflictMarker::Separator, ConflictMarker::Start, 2 },
    missing_separator = { "<<<<<<< HEAD\n- a\n>>>>>>> feature\n", ConflictMarker::Separator, ConflictMarker::End, 3 },
    base_without_separator = { "<<<<<<< HEAD\n||||||| base\n>>>>>>> feature\n", ConflictMarker::Separator, ConflictMarker::End, 3 },
    second_separator = { "<<<<<<< HEAD\n=======\n=======\n", ConflictMarker::End, ConflictMarker::Separator, 3 },
    base_after_separator = { "<<<<<<< HEAD\n=======\n||||||| base\n", ConflictMarker::End, ConflictMarker::Base, 3 },
)]
fn unexpected_markers(input: &str, expected: ConflictMarker, found: ConflictMarker, line: usize) {
    assert_eq!(
        find_conflicts(input),
        Err(ConflictMarkerError::Unexpected {
            expected,
            found,
            line
        })
    );
}

#[parameterized(
    after_start = { "# Changelog\n<<<<<<< HEAD\n- a\n" },
    after_base = { "# Changelog\n<<<<<<< HEAD\n||||||| base\n" },
    after_separator = { "# Changelog\n<<<<<<< HEAD\n- a\n=======\n- b\n" },
)]
fn unterminated(input: &str) {
    assert_eq!(
        find_conflicts(input),
        Err(ConflictMarkerError::Unterminated { line: 2 })
    );
}

#[parameterized(
    same_on_both_sides = {
        "<<<<<<< HEAD\n- a\n=======\n- a\n>>>>>>> feature\n",
        "- a\n",
    },
    ours_unchanged = {
        "<<<<<<< HEAD\n- x\n||||||| base\n- x\n=======\n- y\n>>>>>>> feature\n",
        "- y\n",
    },
    theirs_unchanged = {
        "<<<<<<< HEAD\n- y\n||||||| base\n- x\n=======\n- x\n>>>>>>> feature\n",
        "- y\n",
    },
    entries = {
        "<<<<<<< HEAD\n- a\n=======\n- b\n>>>>>>> feature\n",
        "- a\n- b\n",
    },
    shared_entry = {
        "<<<<<<< HEAD\n- a\n- shared\n=======\n* shared\n- b\n>>>>>>> feature\n",
        "- a\n- shared\n- b\n",
    },
    multi_line_entries = {
        "<<<<<<< HEAD\n- a\n  with more\n- c\n=======\n- b\n  - nested\n>>>>>>> feature\n",
        "- a\n  with more\n- c\n- b\n  - nested\n",
    },
    lazy_continuation = {
        "<<<<<<< HEAD\n- a\ncontinued\n=======\n- b\n>>>>>>> feature\n",
        "- a\ncontinued\n- b\n",
    },
    ordered_entries = {
        "<<<<<<< HEAD\n1. a\n=======\n2) b\n>>>>>>> feature\n",
        "1. a\n2) b\n",
    },
    different_sections = {
        "<<<<<<< HEAD\n### Added\n\n- a\n\n=======\n### Fixed\n\n- f\n\n>>>>>>> feature\n",
        "### Added\n\n- a\n\n### Fixed\n\n- f\n\n",
    },
    same_section = {
        "<<<<<<< HEAD\n\n### Fixed\n\n- a\n=======\n\n### fixed\n\n- b\n>>>>>>> feature\n",
        "\n### Fixed\n\n- a\n- b\n",
    },
    entries_and_a_section = {
        "<<<<<<< HEAD\n\n### Changed\n\n- c\n=======\n- b\n>>>>>>> feature\n",
        "- b\n\n### Changed\n\n- c\n",
    },
    links = {
        "<<<<<<< HEAD\n[#1]: https://example.com/1\n=======\n[#2]: https://example.com/2\n>>>>>>> feature\n",
        "[#1]: https://example.com/1\n[#2]: https://example.com/2\n",
    },
    removed_with_base = {
        "<<<<<<< HEAD\n- a\n- new\n||||||| base\n- a\n- old\n=======\n- a\n- old\n- b\n>>>>>>> feature\n",
        "- a\n- new\n- b\n",
    },
    edited_different_entries = {
        "<<<<<<< HEAD\n- a2\n- x\n||||||| base\n- a\n- x\n=======\n- a\n- x2\n>>>>>>> feature\n",
        "- a2\n- x2\n",
    },
    removed_section = {
        "<<<<<<< HEAD\n### Added\n\n- a\n||||||| base\n### Added\n\n- a\n\n### Fixed\n\n- f\n=======\n### Added\n\n- a\n- b\n\n### Fixed\n\n- f\n>>>>>>> feature\n",
        "### Added\n\n- a\n- b\n",
    },
    one_side_empty = {
        "<<<<<<< HEAD\n=======\n- b\n>>>>>>> feature\n",
        "- b\n",
    },
)]
fn resolves(input: &str, expected: &str) {
    assert_eq!(outcome(input), Ok(expected.to_string()));
}

#[test]
fn resolves_links_with_one_side_changed() {
    let input = "<<<<<<< HEAD\n[1.0]: https://new\n||||||| base\n[1.0]: https://old\n=======\n[1.0]: https://old\n[1.1]: https://b\n>>>>>>> feature\n";

    assert_eq!(
        outcome(input),
        Ok("[1.0]: https://new\n[1.1]: https://b\n".to_string())
    );
}

#[parameterized(
    edited_entry = {
        "<<<<<<< HEAD\n- Fix the parser\n||||||| base\n- Fix parser\n=======\n- Fix a parser bug\n>>>>>>> feature\n",
        UnresolvableReason::EditedEntry { text: "Fix parser".to_string() },
    },
    conflicting_link = {
        "<<<<<<< HEAD\n[#1]: https://a\n=======\n[#1]: https://b\n>>>>>>> feature\n",
        UnresolvableReason::ConflictingLink { label: "#1".to_string() },
    },
    removed_section = {
        "<<<<<<< HEAD\n### Added\n\n- a\n||||||| base\n### Added\n\n- a\n\n### Fixed\n\n- f\n=======\n### Added\n\n- a\n\n### Fixed\n\n- f\n- g\n>>>>>>> feature\n",
        UnresolvableReason::RemovedSection { kind: ChangeKind::Fixed },
    },
    release_heading = {
        "<<<<<<< HEAD\n## [1.0.0] - 2024-01-31\n=======\n- b\n>>>>>>> feature\n",
        UnresolvableReason::Unrecognized { text: "## [1.0.0] - 2024-01-31".to_string() },
    },
    paragraph = {
        "<<<<<<< HEAD\nSome text\n=======\n- b\n>>>>>>> feature\n",
        UnresolvableReason::Unrecognized { text: "Some text".to_string() },
    },
    entry_after_links = {
        "<<<<<<< HEAD\n[#1]: https://a\n- a\n=======\n- b\n>>>>>>> feature\n",
        UnresolvableReason::Unrecognized { text: "- a".to_string() },
    },
    empty_section_name = {
        "<<<<<<< HEAD\n###\n=======\n- b\n>>>>>>> feature\n",
        UnresolvableReason::Unrecognized { text: "###".to_string() },
    },
)]
fn unresolvable(input: &str, expected: UnresolvableReason) {
    assert_eq!(outcome(input), Err(expected));
}

#[test]
fn keeps_crlf_line_endings() {
    let input = "<<<<<<< HEAD\r\n- a\r\n=======\r\n- b\r\n>>>>>>> feature\r\n";

    assert_eq!(outcome(input), Ok("- a\r\n- b\r\n".to_string()));
}

#[test]
fn keeps_a_missing_trailing_newline() {
    let input = "<<<<<<< HEAD\n- a\n=======\n- b\n>>>>>>> feature";

    assert_eq!(outcome(input), Ok("- a\n- b".to_string()));
}

const CONFLICTED: &str = "# Changelog

## [Unreleased]

### Added

- a
<<<<<<< HEAD
- b
=======
- c
>>>>>>> feature

### Fixed

<<<<<<< HEAD
- f
=======
- g
>>>>>>> feature

## [1.0.0] - 2024-01-31

### Added

- Initial release.
";

#[test]
fn resolves_a_changelog() {
    assert_eq!(
        resolve_conflicts(CONFLICTED).unwrap(),
        "# Changelog

## [Unreleased]

### Added

- a
- b
- c

### Fixed

- f
- g

## [1.0.0] - 2024-01-31

### Added

- Initial release.
"
    );
}

#[test]
fn resolve_reports_every_unresolvable_conflict() {
    let input = "# Changelog\n\n<<<<<<< HEAD\nText\n=======\n- b\n>>>>>>> feature\n\n<<<<<<< HEAD\n- a\n=======\n- b\n>>>>>>> feature\n\n<<<<<<< HEAD\n[#1]: https://a\n=======\n[#1]: https://b\n>>>>>>> feature\n";

    let Err(ResolveError::Unresolvable(conflicts)) = resolve_conflicts(input) else {
        panic!("expected unresolvable conflicts");
    };
    let lines: Vec<usize> = conflicts.iter().map(|conflict| conflict.line).collect();
    assert_eq!(lines, [3, 15]);
}

#[test]
fn resolve_reports_marker_errors() {
    assert!(matches!(
        resolve_conflicts("# Changelog\n<<<<<<< HEAD\n"),
        Err(ResolveError::Markers(ConflictMarkerError::Unterminated {
            line: 2
        }))
    ));
}

#[test]
fn resolve_checks_the_result_is_a_valid_changelog() {
    let input = "# Changelog\n\n## [Unreleased]\n\n### Added\n\n- a\n\n<<<<<<< HEAD\n### Added\n\n- b\n=======\n### Fixed\n\n- f\n>>>>>>> feature\n";

    assert!(matches!(
        resolve_conflicts(input),
        Err(ResolveError::Parse(ParseError::Section(_)))
    ));
}

#[test]
fn resolve_without_conflicts_returns_the_input() {
    let input = "# Changelog\n\n## [Unreleased]\n";

    assert_eq!(resolve_conflicts(input).unwrap(), input);
}

const TWO_CONFLICTS: &str = "- x
<<<<<<< HEAD
- a
=======
- b
>>>>>>> feature
- y
<<<<<<< HEAD
- c
=======
- d
>>>>>>> feature
";

#[test]
fn apply_resolutions_applies_only_the_given_conflicts() {
    let conflicts = find_conflicts(TWO_CONFLICTS).unwrap();

    assert_eq!(
        apply_resolutions(TWO_CONFLICTS, &conflicts[1..]),
        "- x
<<<<<<< HEAD
- a
=======
- b
>>>>>>> feature
- y
- c
- d
"
    );
}

#[test]
fn apply_resolutions_in_any_order() {
    let conflicts = find_conflicts(TWO_CONFLICTS).unwrap();

    assert_eq!(
        apply_resolutions(TWO_CONFLICTS, [&conflicts[1], &conflicts[0], &conflicts[1]]),
        "- x\n- a\n- b\n- y\n- c\n- d\n"
    );
}

#[test]
fn apply_resolutions_without_conflicts() {
    assert_eq!(apply_resolutions(TWO_CONFLICTS, []), TWO_CONFLICTS);
}

#[test]
fn apply_resolutions_leaves_unresolvable_conflicts() {
    let input = "<<<<<<< HEAD\n## [1.0.0]\n=======\n## [0.9.0]\n>>>>>>> feature\n";
    let conflicts = find_conflicts(input).unwrap();

    assert_eq!(apply_resolutions(input, &conflicts), input);
}
