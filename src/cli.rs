use std::{
    ffi::OsString,
    fmt::Display,
    process::{Command, Stdio},
    sync::LazyLock,
};

use clap::Parser;
use log::debug;
use regex_lite::Regex;

#[derive(Debug, Parser)]
#[command(version, about)]
pub struct FuzipArgs {
    /// Directories to match files from
    #[arg(value_name = "input", num_args = 2..)]
    pub inputs: Vec<OsString>,
    /// Print commands before executing them
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,
    // TODO: can clap validate this to be non-empty?
    /// The command to execute on pairs
    #[arg(short = 'x', long = "exec")]
    exec: Option<String>,
    /// Don't run command, just show what would be run
    #[arg(short = 'n', long = "dry-run", requires = "exec")]
    pub dry_run: bool,
}

impl FuzipArgs {
    pub fn exec(&self) -> Option<ExecBlueprint> {
        self.exec.as_ref().map(ExecBlueprint::from)
    }
}

pub struct ExecBlueprint(Vec<String>);

impl ExecBlueprint {
    // TODO: newtype that doesn't allow modification, only execution. Maybe
    //       better debug repr
    pub fn to_command(&self, args: &[impl Display]) -> Command {
        static PLACEHOLDER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
            // Captures the number within a {1} placeholder. Requires
            // full-string match
            Regex::new(r"^\{(?P<index>\d+)\}$").unwrap()
        });
        let swap_placeholder = |part: &str| -> String {
            match PLACEHOLDER_REGEX.captures(part) {
                Some(captures) => {
                    let index = captures
                        .name("index")
                        .unwrap()
                        .as_str()
                        .parse::<usize>()
                        .expect("placeholder exceeded usize::MAX");
                    // TODO: error handling here
                    debug!("placeholder {part} => {}", &args[index]);
                    args[index].to_string()
                },
                None => part.to_string(),
            }
        };

        // TODO: this should be type-encoded within ExecBlueprint
        let Some((first, rest)) = self.0.split_first() else {
            panic!("empty ExecBlueprint");
        };
        let mut cmd = Command::new(swap_placeholder(first));
        cmd.args(rest.iter().map(|part| swap_placeholder(part)));
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());
        cmd.stdin(Stdio::null());
        cmd
    }
}

// TODO: maybe use TryFrom here so we can surface the shlex error and any empty
//       string errors
impl<S: AsRef<str>> From<S> for ExecBlueprint {
    fn from(value: S) -> Self {
        ExecBlueprint(shlex::split(value.as_ref()).expect("shlex failed"))
    }
}
