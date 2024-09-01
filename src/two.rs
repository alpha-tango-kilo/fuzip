use std::{iter::FusedIterator, mem};

use log::debug;
use pathfinding::{kuhn_munkres::kuhn_munkres_min, matrix::Matrix};

use crate::{Fuzip, Fuzippable};

pub fn fuzzy_zip_two<'a, T>(
    lefts: &'a [T],
    rights: &'a [T],
) -> impl Iterator<Item = Fuzip<&'a T>>
where
    T: Fuzippable,
{
    debug_assert!(!lefts.is_empty(), "lefts empty");
    debug_assert!(!rights.is_empty(), "rights empty");

    // Re-assign these so they can be swapped if needed, Matrix needs rows <=
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

    let matrix = crate::time!(
        "build matrix",
        Matrix::from_fn(
            lefts.len(),
            rights.len(),
            |(left_index, right_index)| {
                let weight = strsim::generic_damerau_levenshtein(
                    lefts[left_index].key(),
                    rights[right_index].key(),
                );
                i64::try_from(weight).expect("weight unable to fit in i64")
            },
        )
    );

    let (_, assignments) =
        crate::time!("solve with Kuhn-Munkres", kuhn_munkres_min(&matrix));

    Fuzip2Iterator::new(lefts, rights, assignments, swapped)
}

#[derive(Debug)]
struct Fuzip2Iterator<'a, T> {
    lefts: &'a [T],
    // Guaranteed to be longest
    rights: Vec<Option<&'a T>>,
    assignments: Vec<usize>,
    swapped: bool,
}

impl<'a, T> Fuzip2Iterator<'a, T> {
    fn new(
        lefts: &'a [T],
        rights: &'a [T],
        assignments: Vec<usize>,
        swapped: bool,
    ) -> Self {
        Fuzip2Iterator {
            lefts,
            rights: rights.iter().map(Some).collect(),
            assignments,
            swapped,
        }
    }
}

impl<'a, T> Iterator for Fuzip2Iterator<'a, T> {
    type Item = Fuzip<&'a T>;

    fn next(&mut self) -> Option<Self::Item> {
        debug_assert!(self.lefts.len() <= self.rights.len());

        // General plan here: use the length of self.assignments to track the
        // index of the other side. Remove values from rights as we go so it's
        // easy to tell which values are leftovers once we've emptied
        // self.assignments. We know lefts will have all been used
        let next = if !self.assignments.is_empty() {
            // Matched left/right pairs
            if !self.swapped {
                let right_index = self.assignments.pop().unwrap();
                let left_index = self.assignments.len();
                let right = self.rights[right_index].take().unwrap();
                crate::fuzip!(&self.lefts[left_index], right)
            } else {
                let left_index = self.assignments.pop().unwrap();
                let right_index = self.assignments.len();
                let right = self.rights[right_index].take().unwrap();
                crate::fuzip!(&self.lefts[left_index], right)
            }
        } else {
            // Mismatched stragglers
            let next_value = self.rights.iter_mut().find_map(Option::take);
            match next_value {
                Some(value) if !self.swapped => crate::fuzip!(None, value),
                Some(value) => crate::fuzip!(value, None),
                None => return None,
            }
        };
        Some(next)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.rights.len(), Some(self.rights.len()))
    }
}

impl<'a, T> ExactSizeIterator for Fuzip2Iterator<'a, T> {}
impl<'a, T> FusedIterator for Fuzip2Iterator<'a, T> {}

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
