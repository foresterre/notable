use std::io::{BufRead, Write};
use std::path::Path;

use notable_merge::Conflict;

use crate::error::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Answer {
    Yes,
    No,
    All,
    Quit,
}

const HELP: &str = "y - apply this resolution
n - skip this conflict
a - apply this resolution and all remaining ones
q - skip this conflict and all remaining ones
? - print help
";

pub fn choose<'c>(
    path: &Path,
    input: &str,
    conflicts: &'c [Conflict],
    answers: &mut impl BufRead,
    out: &mut impl Write,
) -> Result<Vec<&'c Conflict>, Error> {
    let resolvable: Vec<&Conflict> = conflicts
        .iter()
        .filter(|conflict| conflict.outcome.is_ok())
        .collect();
    let mut accepted = Vec::new();

    for (index, conflict) in resolvable.iter().enumerate() {
        writeln!(
            out,
            "{}:{} ({}/{})",
            path.display(),
            conflict.line,
            index + 1,
            resolvable.len()
        )
        .map_err(Error::Stdout)?;
        let hunk = notable_merge::unified_diff_hunks(input, std::slice::from_ref(*conflict));
        out.write_all(hunk.as_bytes()).map_err(Error::Stdout)?;

        match ask(answers, out)? {
            Answer::Yes => accepted.push(*conflict),
            Answer::No => {}
            Answer::All => {
                accepted.extend(&resolvable[index..]);
                break;
            }
            Answer::Quit => break,
        }
    }

    Ok(accepted)
}

fn ask(answers: &mut impl BufRead, out: &mut impl Write) -> Result<Answer, Error> {
    loop {
        write!(out, "Apply this resolution [y,n,a,q,?]? ").map_err(Error::Stdout)?;
        out.flush().map_err(Error::Stdout)?;

        let mut line = String::new();
        let read = answers.read_line(&mut line).map_err(Error::Stdin)?;
        writeln!(out).map_err(Error::Stdout)?;

        if read == 0 {
            return Ok(Answer::Quit);
        }

        let answer = match line.trim().to_ascii_lowercase().as_str() {
            "y" | "yes" => Answer::Yes,
            "n" | "no" => Answer::No,
            "a" | "all" => Answer::All,
            "q" | "quit" => Answer::Quit,
            _ => {
                out.write_all(HELP.as_bytes()).map_err(Error::Stdout)?;
                continue;
            }
        };
        return Ok(answer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yare::parameterized;

    const INPUT: &str = "# Changelog
<<<<<<< HEAD
- a
=======
- b
>>>>>>> feature
<<<<<<< HEAD
## [1.0.0]
=======
## [0.9.0]
>>>>>>> feature
<<<<<<< HEAD
- c
=======
- d
>>>>>>> feature
<<<<<<< HEAD
- e
=======
- f
>>>>>>> feature
";

    fn chosen(answers: &str) -> (Vec<usize>, String) {
        let conflicts = notable_merge::find_conflicts(INPUT).unwrap();
        let mut out = Vec::new();

        let accepted = choose(
            Path::new("CHANGELOG.md"),
            INPUT,
            &conflicts,
            &mut answers.as_bytes(),
            &mut out,
        )
        .unwrap();

        let lines = accepted.iter().map(|conflict| conflict.line).collect();
        (lines, String::from_utf8(out).unwrap())
    }

    #[parameterized(
        yes_to_all = { "y\ny\ny\n", &[2, 12, 17] },
        no_to_all = { "n\nn\nn\n", &[] },
        mixed = { "y\nn\ny\n", &[2, 17] },
        words = { "Yes\nNO\n yes \n", &[2, 17] },
        all = { "n\na\n", &[12, 17] },
        quit = { "y\nq\n", &[2] },
        end_of_input = { "y\n", &[2] },
        no_input = { "", &[] },
        help_then_answer = { "?\ny\nwhat\nn\ny\n", &[2, 17] },
    )]
    fn answers(answers: &str, expected: &[usize]) {
        assert_eq!(chosen(answers).0, expected);
    }

    #[test]
    fn skips_unresolvable_conflicts() {
        let (_, out) = chosen("n\nn\nn\n");

        assert!(out.contains("CHANGELOG.md:2 (1/3)"));
        assert!(out.contains("CHANGELOG.md:12 (2/3)"));
        assert!(out.contains("CHANGELOG.md:17 (3/3)"));
    }

    #[test]
    fn shows_the_change() {
        let (_, out) = chosen("q\n");

        assert_eq!(
            out,
            "CHANGELOG.md:2 (1/3)
@@ -1,9 +1,6 @@
 # Changelog
-<<<<<<< HEAD
-- a
-=======
-- b
->>>>>>> feature
+- a
+- b
 <<<<<<< HEAD
 ## [1.0.0]
 =======
Apply this resolution [y,n,a,q,?]? \n"
        );
    }

    #[test]
    fn prints_help() {
        let (_, out) = chosen("?\nq\n");

        assert!(out.contains(HELP));
    }
}
