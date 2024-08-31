use std::{
    ffi::OsString,
    process::{Command, Stdio},
    sync::LazyLock,
};

use clap::Parser;
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
    pub fn to_command(&self, args: &[impl ToString]) -> Command {
        static PLACEHOLDER_REGEX: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\{(?P<index>\d+)\}").unwrap());
        let swap_placeholder = |part: &str| -> String {
            match PLACEHOLDER_REGEX.captures(part) {
                Some(captures) => {
                    let index = captures
                        .name("index")
                        .unwrap()
                        .as_str()
                        .parse::<usize>()
                        .expect("placeholder exceeded usize::MAX");
                    args[index].to_string()
                },
                None => part.to_string(),
            }
        };

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

impl<S: AsRef<str>> From<S> for ExecBlueprint {
    fn from(value: S) -> Self {
        ExecBlueprint(shlex::split(value.as_ref()).expect("shlex failed"))
    }
}
