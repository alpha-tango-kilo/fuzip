use std::{
    error::Error, ffi::OsString, fmt, fmt::Write, fs, io,
    os::windows::fs::FileTypeExt, path::PathBuf,
};

use anyhow::bail;
use clap::Parser;
use env_logger::Env;
use log::{error, info, LevelFilter};

use crate::{cli::FuzipArgs, two::fuzzy_zip_two};

mod cli;
mod two;

#[macro_export]
macro_rules! time {
    ($task:literal, $e:expr) => {{
        let now = std::time::Instant::now();
        let expr = $e;
        log::trace!("{} took {:?}", $task, now.elapsed());
        expr
    }};
}

#[macro_export]
macro_rules! fuzip {
    ($($e:expr $(,)?)+) => {{
        $crate::Fuzip(vec![
            $(Option::from($e),)+
        ])
    }};
}

fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .parse_env(Env::new().filter("FUZIP_LOG"))
        .format_timestamp(None)
        .format_module_path(cfg!(debug_assertions))
        .init();

    let args = FuzipArgs::parse();
    let inputs = time!("prep_paths", prep_paths(&args.inputs))?;
    let [lefts, rights] = inputs.as_slice() else {
        bail!("currently only 2 inputs are supported");
    };

    let exec = args.exec();
    // TODO: when zipping paths, use file stems for matching
    fuzzy_zip_two(lefts, rights).try_for_each(
        |fuzip| -> anyhow::Result<()> {
            // TODO: properly use Fuzip
            match &exec {
                Some(exec) => {
                    let mut command = exec.to_command(&[
                        fuzip.get(0).unwrap().display(),
                        fuzip.get(1).unwrap().display(),
                    ])?;
                    if args.verbose || args.dry_run {
                        info!("Running {command:?}");
                    }
                    if !args.dry_run {
                        let status = command.status()?;
                        if !status.success() {
                            error!("exited with code {status}: {command:?}");
                        }
                    }
                },
                None => {
                    println!("{:?} {:?}", fuzip.get(0), fuzip.get(1),);
                },
            }
            Ok(())
        },
    )?;
    Ok(())
}

fn prep_paths(inputs: &[OsString]) -> anyhow::Result<Vec<Vec<PathBuf>>> {
    inputs
        .iter()
        .map(|dir_path| -> anyhow::Result<_> {
            let mut values = Vec::new();
            fs::read_dir(dir_path)?.try_for_each(
                |dir_entry_res| -> io::Result<_> {
                    let dir_entry = dir_entry_res?;
                    let file_type = dir_entry.file_type()?;
                    if file_type.is_file() || file_type.is_symlink_file() {
                        values.push(dir_entry.path());
                    }
                    Ok(())
                },
            )?;
            Ok(values)
        })
        .collect()
}

#[derive(Debug)]
pub struct Fuzip<T>(Vec<Option<T>>);

impl<T> Fuzip<T> {
    pub fn get(&self, index: usize) -> Result<&T, FuzipMissing> {
        match self.0.get(index) {
            Some(Some(t)) => Ok(t),
            Some(None) => Err(FuzipMissing::NoMatch),
            None => Err(FuzipMissing::OutOfBounds),
        }
    }

    pub fn width(&self) -> usize {
        self.0.len()
    }

    pub fn complete(&self) -> bool {
        self.0.iter().all(Option::is_some)
    }
}

impl<T> fmt::Display for Fuzip<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = String::new();
        for t in self.0.iter().filter_map(Option::as_ref) {
            let _ = write!(&mut buf, "{t} ");
        }
        buf.pop();
        f.write_str(&buf)
    }
}

#[derive(Debug)]
pub enum FuzipMissing {
    NoMatch,
    OutOfBounds,
}

impl fmt::Display for FuzipMissing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            FuzipMissing::NoMatch => f.write_str("no matching value"),
            FuzipMissing::OutOfBounds => f.write_str("index out of bounds"),
        }
    }
}

impl Error for FuzipMissing {}
