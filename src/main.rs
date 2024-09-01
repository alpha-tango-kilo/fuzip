use std::{
    error::Error, ffi::OsString, fmt, fmt::Write, fs, hash::Hash, io,
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
    fuzzy_zip_two(lefts, rights)
        .filter(|fuzip| !args.full_only || fuzip.complete())
        .try_for_each(|fuzip| -> anyhow::Result<()> {
            match &exec {
                Some(exec) => {
                    let mut command = exec.to_command(&fuzip)?;
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
                    println!("{fuzip}");
                },
            }
            Ok(())
        })?;
    Ok(())
}

fn prep_paths(inputs: &[OsString]) -> anyhow::Result<Vec<Vec<FuzipPath>>> {
    inputs
        .iter()
        .map(|dir_path| -> anyhow::Result<_> {
            let mut values = Vec::new();
            fs::read_dir(dir_path)?.try_for_each(
                |dir_entry_res| -> io::Result<_> {
                    let dir_entry = dir_entry_res?;
                    let file_type = dir_entry.file_type()?;
                    if file_type.is_file() || file_type.is_symlink_file() {
                        values.push(dir_entry.path().into());
                    }
                    Ok(())
                },
            )?;
            Ok(values)
        })
        .collect()
}

pub trait Fuzippable {
    type Inner;

    /// Access the inner type
    fn get(&self) -> &Self::Inner;

    // Trait bounds from strsim algorithm
    /// The value to be used for fuzzy matching comparison
    fn key(&self) -> &[impl Eq + Hash + Clone];
    // TODO: can I provide these default implementations if the type bounds are
    //       met?
    // where
    //     <Self as Fuzippable>::Inner: Eq + Hash + Clone,
    // {
    //     self.get()
    // }

    /// How to print the item
    fn display(&self) -> impl fmt::Display;
    // where
    //     <Self as Fuzippable>::Inner: fmt::Display,
    // {
    //     self.get()
    // }
}

// FIXME: I don't like that I need this impl, can I do away with it somehow?
impl<T> Fuzippable for &T
where
    T: Fuzippable,
{
    type Inner = T::Inner;

    fn get(&self) -> &Self::Inner {
        (*self).get()
    }

    fn key(&self) -> &[impl Eq + Hash + Clone] {
        (*self).key()
    }

    fn display(&self) -> impl fmt::Display {
        (*self).display()
    }
}

#[derive(Debug, Clone)]
pub struct FuzipPath(PathBuf);

impl Fuzippable for FuzipPath {
    type Inner = PathBuf;

    fn get(&self) -> &Self::Inner {
        &self.0
    }

    fn key(&self) -> &[impl Eq + Hash + Clone] {
        self.0
            .file_stem()
            .expect("FuzipPath with no stem")
            .as_encoded_bytes()
    }

    fn display(&self) -> impl fmt::Display {
        self.0.display()
    }
}

impl From<PathBuf> for FuzipPath {
    fn from(path: PathBuf) -> Self {
        FuzipPath(path)
    }
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
    T: Fuzippable,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = String::new();
        for t in self.0.iter().filter_map(Option::as_ref) {
            let _ = write!(&mut buf, "{} ", t.display());
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
