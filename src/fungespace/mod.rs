/*
rfunge – a Funge-98 interpreter
Copyright © 2021 Thomas Jollans

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as
published by the Free Software Foundation, either version 3 of the
License, or (at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program. If not, see <https://www.gnu.org/licenses/>.
*/

pub mod index;
pub mod paged;

use std::ops::{Index, IndexMut};

pub use self::index::{bfvec, BefungeVec64};
pub use self::paged::PagedFungeSpace;

/// Generic index into funge space. Specific implementations of funge-space
/// require additional traits to be implemented, as do some instructions.
pub trait FungeIndex: Eq + Copy {
    /// Minimum across all components of the index:
    /// Get the largest index for which all components are less than or equal
    /// to the corresponding components of `self` and `other`.
    ///
    /// This is used, e.g., to determine the maximum extent of funge-space.
    fn joint_min(&self, other: &Self) -> Self;

    /// Maximum across all components of the index:
    /// Get the smallest index for which all components are greater than or equal
    /// to the corresponding components of `self` and `other`.
    ///
    /// This is used, e.g., to determine the maximum extent of funge-space.
    fn joint_max(&self, other: &Self) -> Self;
}

/// Trait representing funge-space. Accessing specific
pub trait FungeSpace<Idx>: Index<Idx> + IndexMut<Idx>
where
    Idx: FungeIndex,
{
    /// Move by `delta`, starting from `start`, according to Funge-98 rules.
    ///
    /// Stops at the next non-space character, and returns a tuple of the
    /// index of the new position and (a reference to) the value found there.
    fn move_by(&self, start: Idx, delta: Idx) -> (Idx, &Self::Output);

    /// Get the minimum index with a non-space value, meaning the largest index
    /// such that all data/code is at larger indices.
    /// (See also [FungeIndex::joint_min()])
    ///
    /// Returns `None` when there is no data/code
    fn min_idx(&self) -> Option<Idx>;

    /// Get the minimum index with a non-space value, meaning the largest index
    /// such that all data/code is at larger indices.
    /// (See also [FungeIndex::joint_max()])
    ///
    /// Returns `None` when there is no data/code
    fn max_idx(&self) -> Option<Idx>;
}

/// Trait to help use index types when (part of) funge space is stored in an
/// array
pub trait FungeArrayIdx: FungeIndex {
    /// Get the size of `Vec` needed if `self` is interpreted as the size of
    /// the n-d array. (i.e. the generalized volume, x*y in 2D)
    fn lin_size(&self) -> usize;
    /// Map the index to a linear index into a `Vec` of size `array_size.lin_size()`
    ///
    /// The caller must ensure that the array is big enough.
    fn to_lin_index(&self, array_size: &Self) -> usize;

    /// Convert from a linear index as returned by
    /// [to_lin_index()][FungeArrayIdx::to_lin_index()] back to a funge index.
    ///
    /// The caller must ensure that the array is big enough.
    fn from_lin_index(lin_idx: usize, array_size: &Self) -> Self;
}

/// Read a string into a befunge space
pub fn read_unefunge<FungeSpaceT>(space: &mut FungeSpaceT, src: &str)
where
    FungeSpaceT: FungeSpace<i64>,
    <FungeSpaceT as Index<i64>>::Output: From<i32>,
{
    let mut i = 0;
    for line in src.lines() {
        for c in line.chars() {
            if c != ' ' {
                space[i] = (c as i32).into();
            }
            i += 1;
        }
    }
}

/// Read a string into a befunge space
pub fn read_befunge<FungeSpaceT>(space: &mut FungeSpaceT, src: &str)
where
    FungeSpaceT: FungeSpace<BefungeVec64>,
    <FungeSpaceT as Index<BefungeVec64>>::Output: From<i32>,
{
    for (y, line) in src.lines().enumerate() {
        for (x, c) in line.chars().enumerate() {
            if c != ' ' {
                space[bfvec(x as i64, y as i64)] = (c as i32).into();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Debug;

    // Generic tests for implementors to call
    pub fn test_unefunge_motion<FungeSpaceT, Elem>(space: &mut FungeSpaceT)
    where
        FungeSpaceT: FungeSpace<i64> + Index<i64, Output = Elem>,
        Elem: From<i32> + Eq + Copy + Debug,
    {
        read_unefunge(space, "   3      a  d  ");
        assert_eq!(space[3], Elem::from('3' as i32));
        assert_eq!(space.move_by(3, 1), (10, &Elem::from('a' as i32)));
        assert_eq!(space.move_by(3, -1), (13, &Elem::from('d' as i32)));
        assert_eq!(space.move_by(10, -3), (13, &Elem::from('d' as i32)));
    }

    pub fn test_befunge_motion<FungeSpaceT, Elem>(space: &mut FungeSpaceT)
    where
        FungeSpaceT: FungeSpace<BefungeVec64> + Index<BefungeVec64, Output = Elem>,
        Elem: From<i32> + Eq + Copy + Debug,
    {
        read_befunge(space, "1   5  8\n\n  a b    c\r\n A");

        assert_eq!(space[bfvec(2, 2)], Elem::from('a' as i32));
        assert_eq!(
            space.move_by(bfvec(2, 2), bfvec(1, 1)),
            (bfvec(0, 0), &Elem::from('1' as i32))
        );
        assert_eq!(
            space.move_by(bfvec(2, 2), bfvec(-3, -3)),
            (bfvec(2, 2), &Elem::from('a' as i32))
        );
        assert_eq!(
            space.move_by(bfvec(0, 0), bfvec(-2, 0)),
            (bfvec(4, 0), &Elem::from('5' as i32))
        );
        assert_eq!(
            space.move_by(bfvec(4, 0), bfvec(0, 1)),
            (bfvec(4, 2), &Elem::from('b' as i32))
        );
        assert_eq!(
            space.move_by(bfvec(7, 0), bfvec(2, -1)),
            (bfvec(1, 3), &Elem::from('A' as i32))
        );

        // Try something very far away
        space[bfvec(32000, 8000)] = Elem::from('0' as i32);
        space[bfvec(32000, 2)] = Elem::from('0' as i32);
        assert_eq!(
            space.move_by(bfvec(0, 0), bfvec(4, 1)),
            (bfvec(32000, 8000), &Elem::from('0' as i32))
        );
        assert_eq!(
            space.move_by(bfvec(32000, 8000), bfvec(0, 1)),
            (bfvec(32000, 2), &Elem::from('0' as i32))
        );
        assert_eq!(
            space.move_by(bfvec(32000, 2), bfvec(-1, 0)),
            (bfvec(9, 2), &Elem::from('c' as i32))
        );

        // Check the dimensions
        assert_eq!(space.min_idx(), Some(bfvec(0, 0)));
        assert_eq!(space.max_idx(), Some(bfvec(32000, 8000)));
    }
}
