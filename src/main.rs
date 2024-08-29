use std::{
    ffi::{OsStr, OsString},
    fs, io,
    io::Write,
    os::windows::fs::FileTypeExt,
    path::PathBuf,
};

use anyhow::bail;
use clap::Parser;

use crate::cli::FuzipArgs;

mod cli;

fn main() -> anyhow::Result<()> {
    let args = FuzipArgs::parse();
    let inputs = prep_paths(&args.inputs)?;
    let [lefts, rights] = inputs.as_slice() else {
        bail!("currently only 2 inputs are supported");
    };
    let matched = fuzzy_zip_two(lefts, rights);
    let mut stdout = io::stdout();
    matched.into_iter().for_each(|(left, right)| {
        let _ = writeln!(stdout, "{} {}", left.display(), right.display());
    });
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

fn fuzzy_zip_two<T, U>(_lefts: &[T], _rights: &[U]) -> Vec<(T, U)>
where
    T: AsRef<OsStr>,
    U: AsRef<OsStr>,
{
    todo!()
}
