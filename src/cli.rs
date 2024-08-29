use std::ffi::OsString;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(version, about)]
pub struct FuzipArgs {
    /// Directories to match files from
    #[arg(value_name = "input", num_args = 2..)]
    pub inputs: Vec<OsString>,
}
