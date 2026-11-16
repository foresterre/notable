use std::ops::Range;

use crate::Conflict;

const CONTEXT: usize = 3;

struct Change<'a> {
    old: Range<usize>,
    new: Vec<&'a str>,
}

pub fn unified_diff(path: &str, input: &str, conflicts: &[Conflict]) -> String {
    let hunks = unified_diff_hunks(input, conflicts);
    if hunks.is_empty() {
        return hunks;
    }

    format!("--- a/{path}\n+++ b/{path}\n{hunks}")
}

pub fn unified_diff_hunks(input: &str, conflicts: &[Conflict]) -> String {
    let lines: Vec<&str> = input.split_inclusive('\n').collect();
    let changes: Vec<Change<'_>> = conflicts
        .iter()
        .filter_map(|conflict| {
            let resolved = conflict.outcome.as_ref().ok()?;
            Some(Change {
                old: conflict.line - 1..conflict.end_line,
                new: resolved.split_inclusive('\n').collect(),
            })
        })
        .collect();

    if changes.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    let mut added = 0;
    let mut removed = 0;

    for group in groups(&changes) {
        let (Some(first), Some(last)) = (group.first(), group.last()) else {
            continue;
        };
        let start = first.old.start.saturating_sub(CONTEXT);
        let end = (last.old.end + CONTEXT).min(lines.len());

        let mut body = String::new();
        let mut old_count = 0;
        let mut new_count = 0;
        let mut cursor = start;

        for change in group {
            for line in &lines[cursor..change.old.start] {
                push(&mut body, ' ', line);
            }
            for line in &lines[change.old.clone()] {
                push(&mut body, '-', line);
            }
            for line in &change.new {
                push(&mut body, '+', line);
            }

            old_count += change.old.start - cursor + change.old.len();
            new_count += change.old.start - cursor + change.new.len();
            cursor = change.old.end;
        }
        for line in &lines[cursor..end] {
            push(&mut body, ' ', line);
        }
        old_count += end - cursor;
        new_count += end - cursor;

        let new_start = start + added - removed;
        output.push_str(&format!(
            "@@ -{} +{} @@\n",
            range(start + 1, old_count),
            range(new_start + 1, new_count)
        ));
        output.push_str(&body);

        added += new_count;
        removed += old_count;
    }

    output
}

fn groups<'c, 'a>(changes: &'c [Change<'a>]) -> Vec<&'c [Change<'a>]> {
    let mut groups = Vec::new();
    let mut start = 0;

    for index in 1..changes.len() {
        if changes[index].old.start - changes[index - 1].old.end > 2 * CONTEXT {
            groups.push(&changes[start..index]);
            start = index;
        }
    }
    groups.push(&changes[start..]);

    groups
}

fn range(start: usize, count: usize) -> String {
    match count {
        0 => format!("{},0", start - 1),
        1 => format!("{start}"),
        count => format!("{start},{count}"),
    }
}

fn push(body: &mut String, prefix: char, line: &str) {
    body.push(prefix);
    body.push_str(line);
    if !line.ends_with('\n') {
        body.push_str("\n\\ No newline at end of file\n");
    }
}

#[cfg(test)]
mod tests;
