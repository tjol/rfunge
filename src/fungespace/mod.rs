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

use std::fmt::{Debug, Display};
use std::ops::{AddAssign, DivAssign, MulAssign, RemAssign, SubAssign};
use std::ops::{BitAnd, BitOr, BitXor, Neg, Not};
use std::ops::{BitAndAssign, BitOrAssign, BitXorAssign};
use std::ops::{Index, IndexMut};

use divrem::DivRem;
use num::{FromPrimitive, Num, Signed, ToPrimitive};

pub use self::index::{bfvec, BefungeVec};
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

/// A value that can live in funge space
pub trait FungeValue:
    Num
    + ToPrimitive
    + FromPrimitive
    + From<i32>
    + Signed
    + DivRem<Output = (Self, Self)>
    + BitAnd<Output = Self>
    + BitOr<Output = Self>
    + BitXor<Output = Self>
    + Not<Output = Self>
    + Neg
    + AddAssign
    + SubAssign
    + MulAssign
    + DivAssign
    + RemAssign
    + BitAndAssign
    + BitOrAssign
    + BitXorAssign
    + Ord
    + Eq
    + Copy
    + Display
    + Debug
{
    /// Return the value as a character, if the unicode code point exists
    fn try_to_char(&self) -> Option<char> {
        self.to_u32().and_then(char::from_u32)
    }
    /// Return the value as a character, or U+FFFD �
    fn to_char(&self) -> char {
        match self.try_to_char() {
            Some(c) => c,
            None => '�',
        }
    }
}

impl<T> FungeValue for T where
    T: Num
        + ToPrimitive
        + FromPrimitive
        + From<i32>
        + Signed
        + DivRem<Output = (Self, Self)>
        + BitAnd<Output = Self>
        + BitOr<Output = Self>
        + BitXor<Output = Self>
        + Not<Output = Self>
        + Neg
        + AddAssign
        + SubAssign
        + MulAssign
        + DivAssign
        + RemAssign
        + BitAndAssign
        + BitOrAssign
        + BitXorAssign
        + Ord
        + Eq
        + Copy
        + Display
        + Debug
{
}

pub trait SrcIO<Space>: FungeIndex
where
    Space: FungeSpace<Self>,
{
    fn read_bin_at(space: &mut Space, start: &Self, src: &[u8]);
    fn read_str_at(space: &mut Space, start: &Self, src: &str);
    fn origin() -> Self;
}

/// SrcIO implementation for unefunge
impl<Space, T> SrcIO<Space> for T
where
    T: FungeValue + FungeIndex,
    Space: FungeSpace<T> + Index<T, Output = T>,
{
    /// Read a binary / latin1 file into a unefunge space starting at position `start`
    fn read_bin_at(space: &mut Space, start: &Self, src: &[u8]) {
        let mut idx = *start;

        for byte in src {
            match byte {
                10 | 13 => {} // skip CR & LF
                byte => {
                    let value = *byte as i32;
                    if value != (' ' as i32) {
                        space[idx] = value.into();
                    }
                    idx += 1.into();
                }
            }
        }
    }

    /// Read a string into unifunge space starting at position `start`
    fn read_str_at(space: &mut Space, start: &Self, src: &str) {
        let mut i = *start;

        for line in src.lines() {
            for c in line.chars() {
                if c != ' ' {
                    space[i] = (c as i32).into();
                }
                i += 1.into();
            }
        }
    }

    fn origin() -> Self {
        0.into()
    }
}

/// SrcIO implementation for befunge
impl<Space, T> SrcIO<Space> for BefungeVec<T>
where
    T: FungeValue,
    Space: FungeSpace<BefungeVec<T>> + Index<BefungeVec<T>, Output = T>,
{
    /// Read a binary / latin1 file into a unefunge space starting at position `start`
    fn read_bin_at(space: &mut Space, start: &Self, src: &[u8]) {
        let mut x: T = start.x;
        let mut y: T = start.y;
        let mut recent_cr = false;
        for byte in src {
            match byte {
                10 => {
                    // line feed
                    if !recent_cr {
                        x = start.x;
                        y += 1.into();
                    }
                    recent_cr = false;
                }
                13 => {
                    // carriage return
                    x = start.x;
                    y += 1.into();
                    recent_cr = true;
                }
                byte => {
                    let value = *byte as i32;
                    if value != (' ' as i32) {
                        space[bfvec(x, y)] = value.into();
                    }
                    x += 1.into();
                    recent_cr = false;
                }
            }
        }
    }

    /// Read a string into unifunge space starting at position `start`
    fn read_str_at(space: &mut Space, start: &Self, src: &str) {
        for (y, line) in src.lines().enumerate() {
            for (x, c) in line.chars().enumerate() {
                if c != ' ' {
                    space[*start + bfvec(T::from_usize(x).unwrap(), T::from_usize(y).unwrap())] =
                        (c as i32).into();
                }
            }
        }
    }

    fn origin() -> Self {
        bfvec(0, 0)
    }
}

/// Read a string into a funge space
pub fn read_funge_src<Idx, Space>(space: &mut Space, src: &str)
where
    Space: FungeSpace<Idx>,
    Idx: SrcIO<Space>,
    Space::Output: FungeValue,
{
    Idx::read_str_at(space, &Idx::origin(), src)
}

/// Read a string into a funge space
pub fn read_funge_src_bin<Idx, Space>(space: &mut Space, src: &[u8])
where
    Space: FungeSpace<Idx>,
    Idx: SrcIO<Space>,
    Space::Output: FungeValue,
{
    Idx::read_bin_at(space, &Idx::origin(), src)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Generic tests for implementors to call
    pub fn test_unefunge_motion<T, FungeSpaceT>(space: &mut FungeSpaceT)
    where
        T: FungeValue + FungeIndex,
        FungeSpaceT: FungeSpace<T> + Index<T, Output = T>,
    {
        read_funge_src(space, "   3      a  d  ");
        assert_eq!(space[3.into()], T::from('3' as i32));
        assert_eq!(
            space.move_by(3.into(), 1.into()),
            (10.into(), &T::from('a' as i32))
        );
        assert_eq!(
            space.move_by(3.into(), (-1).into()),
            (13.into(), &T::from('d' as i32))
        );
        assert_eq!(
            space.move_by(10.into(), (-3).into()),
            (13.into(), &T::from('d' as i32))
        );
    }

    pub fn test_befunge_motion<T, FungeSpaceT>(space: &mut FungeSpaceT)
    where
        T: FungeValue,
        FungeSpaceT: FungeSpace<BefungeVec<T>> + Index<BefungeVec<T>, Output = T>,
    {
        read_funge_src(space, "1   5  8\n\n  a b    c\r\n A");

        assert_eq!(space[bfvec(2, 2)], T::from('a' as i32));
        assert_eq!(
            space.move_by(bfvec(2, 2), bfvec(1, 1)),
            (bfvec(0, 0), &T::from('1' as i32))
        );
        assert_eq!(
            space.move_by(bfvec(2, 2), bfvec(-3, -3)),
            (bfvec(2, 2), &T::from('a' as i32))
        );
        assert_eq!(
            space.move_by(bfvec(0, 0), bfvec(-2, 0)),
            (bfvec(4, 0), &T::from('5' as i32))
        );
        assert_eq!(
            space.move_by(bfvec(4, 0), bfvec(0, 1)),
            (bfvec(4, 2), &T::from('b' as i32))
        );
        assert_eq!(
            space.move_by(bfvec(7, 0), bfvec(2, -1)),
            (bfvec(1, 3), &T::from('A' as i32))
        );

        // Try something very far away
        space[bfvec(32000, 8000)] = T::from('0' as i32);
        space[bfvec(32000, 2)] = T::from('0' as i32);
        assert_eq!(
            space.move_by(bfvec(0, 0), bfvec(4, 1)),
            (bfvec(32000, 8000), &T::from('0' as i32))
        );
        assert_eq!(
            space.move_by(bfvec(32000, 8000), bfvec(0, 1)),
            (bfvec(32000, 2), &T::from('0' as i32))
        );
        assert_eq!(
            space.move_by(bfvec(32000, 2), bfvec(-1, 0)),
            (bfvec(9, 2), &T::from('c' as i32))
        );

        // Check the dimensions
        assert_eq!(space.min_idx(), Some(bfvec(0, 0)));
        assert_eq!(space.max_idx(), Some(bfvec(32000, 8000)));
    }
}
