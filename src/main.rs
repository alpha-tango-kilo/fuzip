use std::{
    ffi::{OsStr, OsString},
    fs, io,
    io::Write,
    os::windows::fs::FileTypeExt,
    path::PathBuf,
};

use anyhow::bail;
use clap::Parser;
use pathfinding::{kuhn_munkres::kuhn_munkres_min, matrix::Matrix};

use crate::cli::FuzipArgs;

mod cli;

fn main() -> anyhow::Result<()> {
    let args = FuzipArgs::parse();
    let inputs = prep_paths(&args.inputs)?;
    let [lefts, rights] = inputs.as_slice() else {
        bail!("currently only 2 inputs are supported");
    };
    let mut stdout = io::stdout();
    fuzzy_zip_two(lefts, rights).for_each(|(left, right)| {
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

fn fuzzy_zip_two<'a, L, R>(
    lefts: &'a [L],
    rights: &'a [R],
) -> impl Iterator<Item = (&'a L, &'a R)>
where
    L: AsRef<OsStr>,
    R: AsRef<OsStr>,
{
    debug_assert!(!lefts.is_empty(), "lefts empty");
    debug_assert!(!rights.is_empty(), "rights empty");

    let matrix = Matrix::from_fn(
        lefts.len(),
        rights.len(),
        |(left_index, right_index)| {
            let weight = strsim::generic_damerau_levenshtein(
                lefts[left_index].as_ref().as_encoded_bytes(),
                rights[right_index].as_ref().as_encoded_bytes(),
            );
            i64::try_from(weight).expect("weight unable to fit in i64")
        },
    );

    let (_, assignments) = kuhn_munkres_min(&matrix);
    assignments
        .into_iter()
        .enumerate()
        .map(|(row, column)| (&lefts[row], &rights[column]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zip_two() {
        let answer =
            fuzzy_zip_two(&["aa", "bb"], &["ab", "bb"]).collect::<Vec<_>>();
        dbg!(answer);
    }
}
