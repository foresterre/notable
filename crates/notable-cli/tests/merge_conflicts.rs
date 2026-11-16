mod common;

use common::{ConflictStyle, Instruction, play};
use yare::parameterized;

const CHANGELOG: &str = "CHANGELOG.md";

const BASE: &str = "# Changelog

## [Unreleased]

### Added

- Support for yanked releases.

## [1.0.0] - 2024-01-31

### Added

- Initial release.

[1.0.0]: https://example.com/v1.0.0
";

fn conflicting_branches(
    style: ConflictStyle,
    path: &'static str,
    base: &str,
    ours: &str,
    theirs: &str,
) -> Vec<Instruction> {
    vec![
        Instruction::Init(style),
        Instruction::Write {
            path,
            contents: base.to_string(),
        },
        Instruction::Commit {
            message: "Add a changelog",
        },
        Instruction::Branch { name: "feature" },
        Instruction::Write {
            path,
            contents: theirs.to_string(),
        },
        Instruction::Commit {
            message: "Add their changes",
        },
        Instruction::Switch { name: "main" },
        Instruction::Write {
            path,
            contents: ours.to_string(),
        },
        Instruction::Commit {
            message: "Add our changes",
        },
        Instruction::Merge { branch: "feature" },
    ]
}

#[derive(Debug, Clone, Copy)]
enum Resolution {
    Resolve,
    ApplyDiff,
    Interactive,
}

impl Resolution {
    const ALL: [Resolution; 3] = [
        Resolution::Resolve,
        Resolution::ApplyDiff,
        Resolution::Interactive,
    ];
}

fn resolution(
    how: Resolution,
    path: &'static str,
    args: &[&'static str],
    expected: &str,
) -> Vec<Instruction> {
    match how {
        Resolution::Resolve => resolve(path, args, expected),
        Resolution::ApplyDiff => apply_diff(path, args, expected),
        Resolution::Interactive => interactive(path, args, expected),
    }
}

fn interactive(path: &'static str, args: &[&'static str], expected: &str) -> Vec<Instruction> {
    let mut interactive = vec!["merge-conflicts", "--interactive"];
    interactive.extend(args);

    let mut check = vec!["merge-conflicts"];
    check.extend(args);

    vec![
        Instruction::NotableInput {
            args: interactive,
            stdin: "a\n",
            success: true,
            stderr_contains: "",
        },
        Instruction::AssertFile {
            path,
            contents: expected.to_string(),
        },
        Instruction::Notable {
            args: check,
            success: true,
        },
        Instruction::Git {
            args: vec!["add", path],
            success: true,
        },
        Instruction::Git {
            args: vec!["diff", "--cached", "--check"],
            success: true,
        },
        Instruction::Git {
            args: vec!["commit", "--no-edit"],
            success: true,
        },
    ]
}

fn apply_diff(path: &'static str, args: &[&'static str], expected: &str) -> Vec<Instruction> {
    let mut diff = vec!["merge-conflicts", "--diff"];
    diff.extend(args);

    let mut check = vec!["merge-conflicts"];
    check.extend(args);

    vec![
        Instruction::NotableOutput {
            args: diff,
            path: "resolve.patch",
        },
        Instruction::AssertConflicted { path },
        Instruction::Git {
            args: vec!["apply", "resolve.patch"],
            success: true,
        },
        Instruction::AssertFile {
            path,
            contents: expected.to_string(),
        },
        Instruction::Notable {
            args: check,
            success: true,
        },
        Instruction::Git {
            args: vec!["add", path],
            success: true,
        },
        Instruction::Git {
            args: vec!["diff", "--cached", "--check"],
            success: true,
        },
        Instruction::Git {
            args: vec!["commit", "--no-edit"],
            success: true,
        },
    ]
}

fn resolve(path: &'static str, args: &[&'static str], expected: &str) -> Vec<Instruction> {
    let command = |resolve: bool| -> Vec<&'static str> {
        let mut command = vec!["merge-conflicts"];
        command.extend(args);
        if resolve {
            command.push("--resolve");
        }
        command
    };

    vec![
        Instruction::Notable {
            args: command(false),
            success: true,
        },
        Instruction::AssertConflicted { path },
        Instruction::Notable {
            args: command(true),
            success: true,
        },
        Instruction::AssertFile {
            path,
            contents: expected.to_string(),
        },
        Instruction::Notable {
            args: command(false),
            success: true,
        },
        Instruction::Git {
            args: vec!["add", path],
            success: true,
        },
        Instruction::Git {
            args: vec!["diff", "--cached", "--check"],
            success: true,
        },
        Instruction::Git {
            args: vec!["commit", "--no-edit"],
            success: true,
        },
    ]
}

fn both_add_an_entry(style: ConflictStyle, how: Resolution) -> Vec<Instruction> {
    let ours = BASE.replace(
        "- Support for yanked releases.\n",
        "- Support for yanked releases.\n- Support for custom sections.\n",
    );
    let theirs = BASE.replace(
        "- Support for yanked releases.\n",
        "- Support for yanked releases.\n- A `merge-conflicts` command.\n",
    );
    let expected = BASE.replace(
        "- Support for yanked releases.\n",
        "- Support for yanked releases.\n- Support for custom sections.\n- A `merge-conflicts` command.\n",
    );

    let mut instructions = conflicting_branches(style, CHANGELOG, BASE, &ours, &theirs);
    instructions.extend(resolution(how, CHANGELOG, &[], &expected));
    instructions
}

const EMPTY_UNRELEASED: &str = "# Changelog

## [Unreleased]

## [1.0.0] - 2024-01-31

### Added

- Initial release.
";

fn both_add_a_different_section(style: ConflictStyle, how: Resolution) -> Vec<Instruction> {
    let ours = EMPTY_UNRELEASED.replace(
        "## [Unreleased]\n\n",
        "## [Unreleased]\n\n### Added\n\n- Support for custom sections.\n\n",
    );
    let theirs = EMPTY_UNRELEASED.replace(
        "## [Unreleased]\n\n",
        "## [Unreleased]\n\n### Fixed\n\n- Dates are validated.\n\n",
    );
    let expected = EMPTY_UNRELEASED.replace(
        "## [Unreleased]\n\n",
        "## [Unreleased]\n\n### Added\n\n- Support for custom sections.\n\n### Fixed\n\n- Dates are validated.\n\n",
    );

    let mut instructions = conflicting_branches(style, CHANGELOG, EMPTY_UNRELEASED, &ours, &theirs);
    instructions.extend(resolution(how, CHANGELOG, &[], &expected));
    instructions
}

fn both_add_the_same_section(style: ConflictStyle, how: Resolution) -> Vec<Instruction> {
    let ours = BASE.replace(
        "- Support for yanked releases.\n",
        "- Support for yanked releases.\n\n### Fixed\n\n- Dates are validated.\n",
    );
    let theirs = BASE.replace(
        "- Support for yanked releases.\n",
        "- Support for yanked releases.\n\n### Fixed\n\n- Links in headings are kept.\n",
    );
    let expected = BASE.replace(
        "- Support for yanked releases.\n",
        "- Support for yanked releases.\n\n### Fixed\n\n- Dates are validated.\n- Links in headings are kept.\n",
    );

    let mut instructions = conflicting_branches(style, CHANGELOG, BASE, &ours, &theirs);
    instructions.extend(resolution(how, CHANGELOG, &[], &expected));
    instructions
}

fn both_add_the_same_entry(style: ConflictStyle, how: Resolution) -> Vec<Instruction> {
    let ours = BASE.replace(
        "- Support for yanked releases.\n",
        "- Support for yanked releases.\n- Support for custom sections.\n- Ours.\n",
    );
    let theirs = BASE.replace(
        "- Support for yanked releases.\n",
        "- Support for yanked releases.\n- Theirs.\n- Support for custom sections.\n",
    );
    let expected = BASE.replace(
        "- Support for yanked releases.\n",
        "- Support for yanked releases.\n- Support for custom sections.\n- Ours.\n- Theirs.\n",
    );

    let mut instructions = conflicting_branches(style, CHANGELOG, BASE, &ours, &theirs);
    instructions.extend(resolution(how, CHANGELOG, &[], &expected));
    instructions
}

fn both_add_a_multi_line_entry(style: ConflictStyle, how: Resolution) -> Vec<Instruction> {
    let ours = BASE.replace(
        "- Support for yanked releases.\n",
        "- Support for yanked releases.\n- Support for custom sections.\n  They go after the standard ones.\n",
    );
    let theirs = BASE.replace(
        "- Support for yanked releases.\n",
        "- Support for yanked releases.\n- A `merge-conflicts` command.\n  - With a `--resolve` flag.\n",
    );
    let expected = BASE.replace(
        "- Support for yanked releases.\n",
        "- Support for yanked releases.\n- Support for custom sections.\n  They go after the standard ones.\n- A `merge-conflicts` command.\n  - With a `--resolve` flag.\n",
    );

    let mut instructions = conflicting_branches(style, CHANGELOG, BASE, &ours, &theirs);
    instructions.extend(resolution(how, CHANGELOG, &[], &expected));
    instructions
}

fn one_side_removes_an_entry(style: ConflictStyle, how: Resolution) -> Vec<Instruction> {
    let base = BASE.replace(
        "- Support for yanked releases.\n",
        "- Support for yanked releases.\n- A feature which got reverted.\n",
    );
    let ours = BASE.replace(
        "- Support for yanked releases.\n",
        "- Support for yanked releases.\n- Support for custom sections.\n",
    );
    let theirs = base.replace(
        "- A feature which got reverted.\n",
        "- A feature which got reverted.\n- A `merge-conflicts` command.\n",
    );
    let expected = BASE.replace(
        "- Support for yanked releases.\n",
        "- Support for yanked releases.\n- Support for custom sections.\n- A `merge-conflicts` command.\n",
    );

    let mut instructions = conflicting_branches(style, CHANGELOG, &base, &ours, &theirs);
    instructions.extend(resolution(how, CHANGELOG, &[], &expected));
    instructions
}

struct Changes {
    ours: String,
    theirs: String,
    expected: String,
}

fn both_add_a_link_changes() -> Changes {
    let ours = BASE
        .replace(
            "- Support for yanked releases.\n",
            "- Support for yanked releases.\n- Support for custom sections ([#12]).\n",
        )
        .replace(
            "[1.0.0]: https://example.com/v1.0.0\n",
            "[1.0.0]: https://example.com/v1.0.0\n[#12]: https://example.com/pull/12\n",
        );
    let theirs = BASE
        .replace(
            "- Support for yanked releases.\n",
            "- Support for yanked releases.\n- A `merge-conflicts` command ([#13]).\n",
        )
        .replace(
            "[1.0.0]: https://example.com/v1.0.0\n",
            "[1.0.0]: https://example.com/v1.0.0\n[#13]: https://example.com/pull/13\n",
        );
    let expected = BASE
        .replace(
            "- Support for yanked releases.\n",
            "- Support for yanked releases.\n- Support for custom sections ([#12]).\n- A `merge-conflicts` command ([#13]).\n",
        )
        .replace(
            "[1.0.0]: https://example.com/v1.0.0\n",
            "[1.0.0]: https://example.com/v1.0.0\n[#12]: https://example.com/pull/12\n[#13]: https://example.com/pull/13\n",
        );

    Changes {
        ours,
        theirs,
        expected,
    }
}

fn both_add_a_link(style: ConflictStyle, how: Resolution) -> Vec<Instruction> {
    let changes = both_add_a_link_changes();

    let mut instructions =
        conflicting_branches(style, CHANGELOG, BASE, &changes.ours, &changes.theirs);
    instructions.extend(resolution(how, CHANGELOG, &[], &changes.expected));
    instructions
}

fn apply_the_diff_one_hunk_at_a_time(style: ConflictStyle) -> Vec<Instruction> {
    let changes = both_add_a_link_changes();

    let mut instructions =
        conflicting_branches(style, CHANGELOG, BASE, &changes.ours, &changes.theirs);
    instructions.extend([
        Instruction::NotableOutput {
            args: vec!["merge-conflicts", "--diff"],
            path: "resolve.patch",
        },
        Instruction::ApplyHunks {
            patch: "resolve.patch",
            hunks: 2,
        },
        Instruction::AssertFile {
            path: CHANGELOG,
            contents: changes.expected,
        },
        Instruction::Git {
            args: vec!["add", CHANGELOG],
            success: true,
        },
        Instruction::Git {
            args: vec!["diff", "--cached", "--check"],
            success: true,
        },
        Instruction::Git {
            args: vec!["commit", "--no-edit"],
            success: true,
        },
    ]);
    instructions
}

fn resolve_interactively_one_case_at_a_time(style: ConflictStyle) -> Vec<Instruction> {
    let changes = both_add_a_link_changes();

    let mut instructions =
        conflicting_branches(style, CHANGELOG, BASE, &changes.ours, &changes.theirs);
    instructions.extend([
        Instruction::NotableInput {
            args: vec!["merge-conflicts", "--interactive"],
            stdin: "n\ny\n",
            success: true,
            stderr_contains: "",
        },
        Instruction::AssertConflicted { path: CHANGELOG },
        Instruction::AssertFileContains {
            path: CHANGELOG,
            text: "[#12]: https://example.com/pull/12\n[#13]: https://example.com/pull/13\n",
        },
        Instruction::NotableInput {
            args: vec!["merge-conflicts", "--interactive"],
            stdin: "y\n",
            success: true,
            stderr_contains: "",
        },
        Instruction::AssertFile {
            path: CHANGELOG,
            contents: changes.expected,
        },
        Instruction::Git {
            args: vec!["add", CHANGELOG],
            success: true,
        },
        Instruction::Git {
            args: vec!["diff", "--cached", "--check"],
            success: true,
        },
        Instruction::Git {
            args: vec!["commit", "--no-edit"],
            success: true,
        },
    ]);
    instructions
}

fn resolve_interactively_around_an_unresolvable_case(style: ConflictStyle) -> Vec<Instruction> {
    let edit = |text: &str| -> String { BASE.replace("- Support for yanked releases.\n", text) };
    let ours = edit("- Support for releases which were yanked.\n").replace(
        "[1.0.0]: https://example.com/v1.0.0\n",
        "[1.0.0]: https://example.com/v1.0.0\n[#12]: https://example.com/pull/12\n",
    );
    let theirs = edit("- Support for `[YANKED]` releases.\n").replace(
        "[1.0.0]: https://example.com/v1.0.0\n",
        "[1.0.0]: https://example.com/v1.0.0\n[#13]: https://example.com/pull/13\n",
    );

    let mut instructions = conflicting_branches(style, CHANGELOG, BASE, &ours, &theirs);
    instructions.extend([
        Instruction::NotableInput {
            args: vec!["merge-conflicts", "--interactive"],
            stdin: "y\n",
            success: false,
            stderr_contains: "both sides changed the entry `Support for yanked releases.`",
        },
        Instruction::AssertConflicted { path: CHANGELOG },
        Instruction::AssertFileContains {
            path: CHANGELOG,
            text: "[#12]: https://example.com/pull/12\n[#13]: https://example.com/pull/13\n",
        },
    ]);
    instructions
}

fn custom_path(style: ConflictStyle, how: Resolution) -> Vec<Instruction> {
    const PATH: &str = "docs/CHANGELOG.md";

    let ours = BASE.replace(
        "- Support for yanked releases.\n",
        "- Support for yanked releases.\n- Support for custom sections.\n",
    );
    let theirs = BASE.replace(
        "- Support for yanked releases.\n",
        "- Support for yanked releases.\n- A `merge-conflicts` command.\n",
    );
    let expected = BASE.replace(
        "- Support for yanked releases.\n",
        "- Support for yanked releases.\n- Support for custom sections.\n- A `merge-conflicts` command.\n",
    );

    let mut instructions = conflicting_branches(style, PATH, BASE, &ours, &theirs);
    instructions.extend(resolution(how, PATH, &[PATH], &expected));
    instructions
}

fn crlf_line_endings(style: ConflictStyle, how: Resolution) -> Vec<Instruction> {
    let base = BASE.replace('\n', "\r\n");
    let ours = base.replace(
        "- Support for yanked releases.\r\n",
        "- Support for yanked releases.\r\n- Support for custom sections.\r\n",
    );
    let theirs = base.replace(
        "- Support for yanked releases.\r\n",
        "- Support for yanked releases.\r\n- A `merge-conflicts` command.\r\n",
    );
    let expected = base.replace(
        "- Support for yanked releases.\r\n",
        "- Support for yanked releases.\r\n- Support for custom sections.\r\n- A `merge-conflicts` command.\r\n",
    );

    let mut instructions = conflicting_branches(style, CHANGELOG, &base, &ours, &theirs);
    instructions.extend(resolution(how, CHANGELOG, &[], &expected));
    instructions
}

fn both_edit_the_same_entry(style: ConflictStyle) -> Vec<Instruction> {
    let ours = BASE.replace(
        "- Support for yanked releases.\n",
        "- Support for releases which were yanked.\n",
    );
    let theirs = BASE.replace(
        "- Support for yanked releases.\n",
        "- Support for `[YANKED]` releases.\n",
    );

    let mut instructions = conflicting_branches(style, CHANGELOG, BASE, &ours, &theirs);
    instructions.extend([
        Instruction::Notable {
            args: vec!["merge-conflicts"],
            success: false,
        },
        Instruction::Notable {
            args: vec!["merge-conflicts", "--resolve"],
            success: false,
        },
        Instruction::Notable {
            args: vec!["merge-conflicts", "--diff"],
            success: false,
        },
        Instruction::NotableInput {
            args: vec!["merge-conflicts", "--interactive"],
            stdin: "",
            success: false,
            stderr_contains: "both sides changed the entry `Support for yanked releases.`",
        },
        Instruction::AssertConflicted { path: CHANGELOG },
        Instruction::Git {
            args: vec!["add", CHANGELOG],
            success: true,
        },
        Instruction::Git {
            args: vec!["diff", "--cached", "--check"],
            success: false,
        },
    ]);
    instructions
}

#[parameterized(
    merge = { ConflictStyle::Merge },
    diff3 = { ConflictStyle::Diff3 },
    zdiff3 = { ConflictStyle::Zdiff3 },
)]
fn resolves_both_adding_an_entry(style: ConflictStyle) {
    for how in Resolution::ALL {
        play(&both_add_an_entry(style, how));
    }
}

#[parameterized(
    merge = { ConflictStyle::Merge },
    diff3 = { ConflictStyle::Diff3 },
    zdiff3 = { ConflictStyle::Zdiff3 },
)]
fn resolves_both_adding_a_different_section(style: ConflictStyle) {
    for how in Resolution::ALL {
        play(&both_add_a_different_section(style, how));
    }
}

#[parameterized(
    merge = { ConflictStyle::Merge },
    diff3 = { ConflictStyle::Diff3 },
    zdiff3 = { ConflictStyle::Zdiff3 },
)]
fn resolves_both_adding_the_same_section(style: ConflictStyle) {
    for how in Resolution::ALL {
        play(&both_add_the_same_section(style, how));
    }
}

#[parameterized(
    merge = { ConflictStyle::Merge },
    diff3 = { ConflictStyle::Diff3 },
    zdiff3 = { ConflictStyle::Zdiff3 },
)]
fn resolves_both_adding_the_same_entry(style: ConflictStyle) {
    for how in Resolution::ALL {
        play(&both_add_the_same_entry(style, how));
    }
}

#[parameterized(
    merge = { ConflictStyle::Merge },
    diff3 = { ConflictStyle::Diff3 },
    zdiff3 = { ConflictStyle::Zdiff3 },
)]
fn resolves_both_adding_a_multi_line_entry(style: ConflictStyle) {
    for how in Resolution::ALL {
        play(&both_add_a_multi_line_entry(style, how));
    }
}

#[parameterized(
    diff3 = { ConflictStyle::Diff3 },
    zdiff3 = { ConflictStyle::Zdiff3 },
)]
fn resolves_one_side_removing_an_entry(style: ConflictStyle) {
    for how in Resolution::ALL {
        play(&one_side_removes_an_entry(style, how));
    }
}

#[parameterized(
    merge = { ConflictStyle::Merge },
    diff3 = { ConflictStyle::Diff3 },
    zdiff3 = { ConflictStyle::Zdiff3 },
)]
fn resolves_both_adding_a_link(style: ConflictStyle) {
    for how in Resolution::ALL {
        play(&both_add_a_link(style, how));
    }
}

#[parameterized(
    merge = { ConflictStyle::Merge },
    diff3 = { ConflictStyle::Diff3 },
    zdiff3 = { ConflictStyle::Zdiff3 },
)]
fn resolves_a_custom_path(style: ConflictStyle) {
    for how in Resolution::ALL {
        play(&custom_path(style, how));
    }
}

#[parameterized(
    merge = { ConflictStyle::Merge },
    diff3 = { ConflictStyle::Diff3 },
    zdiff3 = { ConflictStyle::Zdiff3 },
)]
fn resolves_crlf_line_endings(style: ConflictStyle) {
    for how in Resolution::ALL {
        play(&crlf_line_endings(style, how));
    }
}

#[parameterized(
    diff3 = { ConflictStyle::Diff3 },
    zdiff3 = { ConflictStyle::Zdiff3 },
)]
fn refuses_both_editing_the_same_entry(style: ConflictStyle) {
    play(&both_edit_the_same_entry(style));
}

#[parameterized(
    merge = { ConflictStyle::Merge },
    diff3 = { ConflictStyle::Diff3 },
    zdiff3 = { ConflictStyle::Zdiff3 },
)]
fn applies_the_diff_one_hunk_at_a_time(style: ConflictStyle) {
    play(&apply_the_diff_one_hunk_at_a_time(style));
}

#[parameterized(
    merge = { ConflictStyle::Merge },
    diff3 = { ConflictStyle::Diff3 },
    zdiff3 = { ConflictStyle::Zdiff3 },
)]
fn resolves_interactively_one_case_at_a_time(style: ConflictStyle) {
    play(&resolve_interactively_one_case_at_a_time(style));
}

#[parameterized(
    diff3 = { ConflictStyle::Diff3 },
    zdiff3 = { ConflictStyle::Zdiff3 },
)]
fn resolves_interactively_around_an_unresolvable_case(style: ConflictStyle) {
    play(&resolve_interactively_around_an_unresolvable_case(style));
}
