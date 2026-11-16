use super::*;
use crate::find_conflicts;

fn diff(input: &str) -> String {
    unified_diff("CHANGELOG.md", input, &find_conflicts(input).unwrap())
}

#[test]
fn without_conflicts() {
    assert_eq!(diff("# Changelog\n"), "");
}

#[test]
fn single_conflict() {
    let input = "# Changelog

## [Unreleased]

### Added

- a
<<<<<<< HEAD
- b
=======
- c
>>>>>>> feature

## [1.0.0] - 2024-01-31
";

    let expected = [
        "--- a/CHANGELOG.md",
        "+++ b/CHANGELOG.md",
        "@@ -5,10 +5,7 @@",
        " ### Added",
        " ",
        " - a",
        "-<<<<<<< HEAD",
        "-- b",
        "-=======",
        "-- c",
        "->>>>>>> feature",
        "+- b",
        "+- c",
        " ",
        " ## [1.0.0] - 2024-01-31",
        "",
    ];

    assert_eq!(diff(input), expected.join("\n"));
}

#[test]
fn conflicts_far_apart_get_their_own_hunk() {
    let input = "<<<<<<< HEAD
- a
=======
- b
>>>>>>> feature
1
2
3
4
5
6
7
<<<<<<< HEAD
[#1]: https://a
=======
[#2]: https://b
>>>>>>> feature
";

    assert_eq!(
        diff(input),
        "--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -1,8 +1,5 @@
-<<<<<<< HEAD
-- a
-=======
-- b
->>>>>>> feature
+- a
+- b
 1
 2
 3
@@ -10,8 +7,5 @@
 5
 6
 7
-<<<<<<< HEAD
-[#1]: https://a
-=======
-[#2]: https://b
->>>>>>> feature
+[#1]: https://a
+[#2]: https://b
"
    );
}

#[test]
fn conflicts_close_together_share_a_hunk() {
    let input = "<<<<<<< HEAD
- a
=======
- b
>>>>>>> feature
1
<<<<<<< HEAD
- c
=======
- d
>>>>>>> feature
";

    assert_eq!(
        diff(input),
        "--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -1,11 +1,5 @@
-<<<<<<< HEAD
-- a
-=======
-- b
->>>>>>> feature
+- a
+- b
 1
-<<<<<<< HEAD
-- c
-=======
-- d
->>>>>>> feature
+- c
+- d
"
    );
}

#[test]
fn leaves_out_unresolvable_conflicts() {
    let input = "<<<<<<< HEAD
## [1.0.0]
=======
## [0.9.0]
>>>>>>> feature
1
2
3
4
5
6
7
<<<<<<< HEAD
- a
=======
- b
>>>>>>> feature
";

    assert_eq!(
        diff(input),
        "--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -10,8 +10,5 @@
 5
 6
 7
-<<<<<<< HEAD
-- a
-=======
-- b
->>>>>>> feature
+- a
+- b
"
    );
}

#[test]
fn conflict_removed_entirely() {
    let input = "1
<<<<<<< HEAD
=======
>>>>>>> feature
";

    assert_eq!(
        diff(input),
        "--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -1,4 +1 @@
 1
-<<<<<<< HEAD
-=======
->>>>>>> feature
"
    );
}

#[test]
fn keeps_crlf_line_endings() {
    let input = "<<<<<<< HEAD\r\n- a\r\n=======\r\n- b\r\n>>>>>>> feature\r\n";

    assert_eq!(
        diff(input),
        "--- a/CHANGELOG.md\n+++ b/CHANGELOG.md\n@@ -1,5 +1,2 @@\n-<<<<<<< HEAD\r\n-- a\r\n-=======\r\n-- b\r\n->>>>>>> feature\r\n+- a\r\n+- b\r\n"
    );
}

#[test]
fn marks_a_missing_trailing_newline() {
    let input = "<<<<<<< HEAD\n- a\n=======\n- b\n>>>>>>> feature";

    assert_eq!(
        diff(input),
        "--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -1,5 +1,2 @@
-<<<<<<< HEAD
-- a
-=======
-- b
->>>>>>> feature
\\ No newline at end of file
+- a
+- b
\\ No newline at end of file
"
    );
}
