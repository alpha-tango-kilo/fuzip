use std::{
    ffi::{OsStr, OsString},
    fmt, io,
    ops::Deref,
    process::{Command, ExitStatus, Stdio},
    sync::LazyLock,
};

use clap::{builder::NonEmptyStringValueParser, Parser};
use log::debug;
use regex_lite::Regex;
use shlex::Shlex;

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
    #[arg(short = 'x', long = "exec", value_parser = NonEmptyStringValueParser::new())]
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

#[derive(Debug)]
pub struct ExecBlueprint {
    program: String,
    args: Vec<String>,
}

impl ExecBlueprint {
    pub fn to_command(&self, args: &[impl fmt::Display]) -> PreparedCommand {
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

        let mut cmd = Command::new(swap_placeholder(&self.program));
        cmd.args(self.args.iter().map(|part| swap_placeholder(part)));
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());
        cmd.stdin(Stdio::null());
        cmd.into()
    }
}

impl<S> From<S> for ExecBlueprint
where
    S: AsRef<str>,
{
    fn from(invocation: S) -> Self {
        let invocation = invocation.as_ref();
        let mut shell_word_iter = Shlex::new(invocation);
        let program = shell_word_iter
            .next()
            .expect("clap was meant to verify the command wasn't empty");
        let args = shell_word_iter.collect();
        ExecBlueprint { program, args }
    }
}

pub struct PreparedCommand(Command);

impl PreparedCommand {
    pub fn status(&mut self) -> io::Result<ExitStatus> {
        self.0.status()
    }
}

impl Deref for PreparedCommand {
    type Target = Command;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Command> for PreparedCommand {
    fn from(cmd: Command) -> Self {
        PreparedCommand(cmd)
    }
}

impl fmt::Debug for PreparedCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Mostly taken from get-it-going
        let program = self.0.get_program().to_string_lossy();
        let args = self
            .0
            .get_args()
            .map(OsStr::to_string_lossy)
            .collect::<Vec<_>>()
            .join(" ");
        write!(
            f,
            "`{program}{space}{args}`",
            space = if !args.is_empty() { " " } else { "" },
        )
    }
}
