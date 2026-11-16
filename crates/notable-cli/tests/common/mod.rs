use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use assert_fs::TempDir;
use assert_fs::prelude::*;

#[derive(Debug, Clone, Copy)]
pub enum ConflictStyle {
    Merge,
    Diff3,
    Zdiff3,
}

impl ConflictStyle {
    fn as_str(self) -> &'static str {
        match self {
            ConflictStyle::Merge => "merge",
            ConflictStyle::Diff3 => "diff3",
            ConflictStyle::Zdiff3 => "zdiff3",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Init(ConflictStyle),
    Write {
        path: &'static str,
        contents: String,
    },
    Commit {
        message: &'static str,
    },
    Branch {
        name: &'static str,
    },
    Switch {
        name: &'static str,
    },
    Merge {
        branch: &'static str,
    },
    Notable {
        args: Vec<&'static str>,
        success: bool,
    },
    NotableOutput {
        args: Vec<&'static str>,
        path: &'static str,
    },
    ApplyHunks {
        patch: &'static str,
        hunks: usize,
    },
    AssertFile {
        path: &'static str,
        contents: String,
    },
    AssertConflicted {
        path: &'static str,
    },
    AssertFileContains {
        path: &'static str,
        text: &'static str,
    },
    NotableInput {
        args: Vec<&'static str>,
        stdin: &'static str,
        success: bool,
        stderr_contains: &'static str,
    },
    Git {
        args: Vec<&'static str>,
        success: bool,
    },
}

pub fn play(instructions: &[Instruction]) {
    let dir = TempDir::new().unwrap();
    let repo = dir.child("repo");
    repo.create_dir_all().unwrap();
    let global_config = dir.child("gitconfig");
    global_config.touch().unwrap();

    let player = Player {
        repo: repo.path().to_path_buf(),
        global_config: global_config.path().to_path_buf(),
    };

    for instruction in instructions {
        player.play(instruction);
    }
}

struct Player {
    repo: PathBuf,
    global_config: PathBuf,
}

impl Player {
    fn play(&self, instruction: &Instruction) {
        match instruction {
            Instruction::Init(style) => {
                self.git(&["init", "--initial-branch", "main"], true);
                self.git(&["config", "merge.conflictStyle", style.as_str()], true);
            }
            Instruction::Write { path, contents } => {
                let path = self.repo.join(path);
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).unwrap();
                }
                fs::write(path, contents).unwrap();
            }
            Instruction::Commit { message } => {
                self.git(&["add", "--all"], true);
                self.git(&["commit", "--message", message], true);
            }
            Instruction::Branch { name } => {
                self.git(&["switch", "--create", name], true);
            }
            Instruction::Switch { name } => {
                self.git(&["switch", name], true);
            }
            Instruction::Merge { branch } => {
                self.git(&["merge", branch], false);
                let unmerged = self.git(&["ls-files", "--unmerged"], true);
                assert!(
                    !unmerged.stdout.is_empty(),
                    "expected `git merge {branch}` to leave merge conflicts"
                );
            }
            Instruction::Notable { args, success } => {
                let mut command = Command::new(env!("CARGO_BIN_EXE_notable"));
                command.current_dir(&self.repo).args(args);
                run(&mut command, "", *success);
            }
            Instruction::NotableInput {
                args,
                stdin,
                success,
                stderr_contains,
            } => {
                let mut command = Command::new(env!("CARGO_BIN_EXE_notable"));
                command.current_dir(&self.repo).args(args);
                let output = run(&mut command, stdin, *success);
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert!(
                    stderr.contains(stderr_contains),
                    "expected stderr to contain `{stderr_contains}`, found:\n{stderr}"
                );
            }
            Instruction::AssertFileContains { path, text } => {
                let actual = self.read(path);
                assert!(
                    actual.contains(text),
                    "expected `{path}` to contain `{text}`, found:\n{actual}"
                );
            }
            Instruction::NotableOutput { args, path } => {
                let mut command = Command::new(env!("CARGO_BIN_EXE_notable"));
                command.current_dir(&self.repo).args(args);
                let output = run(&mut command, "", true);
                fs::write(self.repo.join(path), output.stdout).unwrap();
            }
            Instruction::ApplyHunks { patch, hunks } => {
                let (header, split) = split_hunks(&self.read(patch));
                assert_eq!(
                    split.len(),
                    *hunks,
                    "unexpected number of hunks in `{patch}`"
                );

                for hunk in split {
                    fs::write(self.repo.join("hunk.patch"), format!("{header}{hunk}")).unwrap();
                    self.git(&["apply", "hunk.patch"], true);
                }
            }
            Instruction::AssertFile { path, contents } => {
                assert_eq!(
                    &self.read(path),
                    contents,
                    "unexpected contents of `{path}`"
                );
            }
            Instruction::AssertConflicted { path } => {
                let actual = self.read(path);
                assert!(
                    actual.contains("<<<<<<< ") && actual.contains(">>>>>>> "),
                    "expected `{path}` to still have merge conflicts, found:\n{actual}"
                );
            }
            Instruction::Git { args, success } => {
                self.git(args, *success);
            }
        }
    }

    fn read(&self, path: &str) -> String {
        fs::read_to_string(self.repo.join(path)).unwrap()
    }

    fn git(&self, args: &[&str], success: bool) -> Output {
        let mut command = Command::new("git");
        command
            .current_dir(&self.repo)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", &self.global_config)
            .env("GIT_AUTHOR_NAME", "Notable")
            .env("GIT_AUTHOR_EMAIL", "notable@example.com")
            .env("GIT_COMMITTER_NAME", "Notable")
            .env("GIT_COMMITTER_EMAIL", "notable@example.com")
            .env("GIT_MERGE_AUTOEDIT", "no")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .args([
                "-c",
                "core.autocrlf=false",
                "-c",
                "core.whitespace=cr-at-eol",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args);

        run(&mut command, "", success)
    }
}

fn split_hunks(patch: &str) -> (String, Vec<String>) {
    let mut header = String::new();
    let mut hunks: Vec<String> = Vec::new();

    for line in patch.split_inclusive('\n') {
        match hunks.last_mut() {
            _ if line.starts_with("@@ ") => hunks.push(line.to_string()),
            Some(hunk) => hunk.push_str(line),
            None => header.push_str(line),
        }
    }

    (header, hunks)
}

fn run(command: &mut Command, stdin: &str, success: bool) -> Output {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(mut input) = child.stdin.take() {
        input.write_all(stdin.as_bytes()).unwrap();
    }
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        output.status.success(),
        success,
        "expected `{}` {} to {}\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        Path::new(command.get_program()).display(),
        command
            .get_args()
            .map(|arg| arg.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" "),
        if success { "succeed" } else { "fail" },
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    output
}
