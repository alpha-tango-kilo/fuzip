use std::{
    ffi::{OsStr, OsString},
    fs, io, mem,
    os::windows::fs::FileTypeExt,
    path::PathBuf,
};

use anyhow::bail;
use clap::Parser;
use env_logger::Env;
use log::{debug, error, info, LevelFilter};
use pathfinding::{kuhn_munkres::kuhn_munkres_min, matrix::Matrix};

use crate::cli::FuzipArgs;

mod cli;

macro_rules! time {
    ($task:literal, $e:expr) => {{
        let now = std::time::Instant::now();
        let expr = $e;
        log::trace!("{} took {:?}", $task, now.elapsed());
        expr
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
    fuzzy_zip_two(lefts, rights).try_for_each(
        |(left, right)| -> anyhow::Result<()> {
            match &exec {
                Some(exec) => {
                    let mut command =
                        exec.to_command(&[left.display(), right.display()]);
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
                    println!("{} {}", left.display(), right.display());
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

fn fuzzy_zip_two<'a, T>(
    lefts: &'a [T],
    rights: &'a [T],
) -> impl Iterator<Item = (&'a T, &'a T)>
where
    T: AsRef<OsStr>,
{
    debug_assert!(!lefts.is_empty(), "lefts empty");
    debug_assert!(!rights.is_empty(), "rights empty");

    // Re-assign these so they can be swapped if needed, Matrix needs rows >=
    // columns
    let mut lefts = lefts;
    let mut rights = rights;
    let swapped = if lefts.len() <= rights.len() {
        false
    } else {
        debug!("swapping lefts & rights");
        mem::swap(&mut lefts, &mut rights);
        true
    };

    let matrix = time!(
        "build matrix",
        Matrix::from_fn(
            lefts.len(),
            rights.len(),
            |(left_index, right_index)| {
                let weight = strsim::generic_damerau_levenshtein(
                    lefts[left_index].as_ref().as_encoded_bytes(),
                    rights[right_index].as_ref().as_encoded_bytes(),
                );
                i64::try_from(weight).expect("weight unable to fit in i64")
            },
        )
    );

    // TODO: return Nones for things that couldn't be matched to, abstraction
    //       type?
    let (_, assignments) =
        time!("solve with Kuhn-Munkres", kuhn_munkres_min(&matrix));
    assignments
        .into_iter()
        .enumerate()
        .map(move |(row, column)| {
            if !swapped {
                (&lefts[row], &rights[column])
            } else {
                (&rights[column], &lefts[row])
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zip_two() {
        let answer = fuzzy_zip_two(&["aa", "bb", "cc"], &["ab", "bb"])
            .collect::<Vec<_>>();
        dbg!(answer);
    }
}
