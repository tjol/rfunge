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

use divrem::DivRem;
use num::{FromPrimitive, Signed, ToPrimitive};
use std::fmt::{Debug, Display};
use std::ops::{
    Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Neg, Rem, RemAssign, Sub,
    SubAssign,
};

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
    From<i32>
    + ToPrimitive
    + FromPrimitive
    + Signed
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Rem<Output = Self>
    + Neg
    + AddAssign
    + SubAssign
    + MulAssign
    + DivAssign
    + RemAssign
    + DivRem<Output = (Self, Self)>
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

impl FungeValue for i32 {}
impl FungeValue for i64 {}
impl FungeValue for i128 {}

/// Read a string into a befunge space
pub fn read_unefunge<T, FungeSpaceT>(space: &mut FungeSpaceT, src: &str)
where
    T: FungeValue + FungeIndex,
    FungeSpaceT: FungeSpace<T> + Index<T, Output = T>,
{
    let mut i = 0;
    for line in src.lines() {
        for c in line.chars() {
            if c != ' ' {
                space[i.into()] = (c as i32).into();
            }
            i += 1;
        }
    }
}

/// Read a string into a befunge space
pub fn read_befunge<T, FungeSpaceT>(space: &mut FungeSpaceT, src: &str)
where
    T: FungeValue,
    FungeSpaceT: FungeSpace<BefungeVec<T>> + Index<BefungeVec<T>, Output = T>,
{
    for (y, line) in src.lines().enumerate() {
        for (x, c) in line.chars().enumerate() {
            if c != ' ' {
                space[bfvec(T::from_usize(x).unwrap(), T::from_usize(y).unwrap())] =
                    (c as i32).into();
            }
        }
    }
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
        read_unefunge(space, "   3      a  d  ");
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
        read_befunge(space, "1   5  8\n\n  a b    c\r\n A");

        assert_eq!(space[bfvec(2.into(), 2.into())], T::from('a' as i32));
        assert_eq!(
            space.move_by(bfvec(2.into(), 2.into()), bfvec(1.into(), 1.into())),
            (bfvec(0.into(), 0.into()), &T::from('1' as i32))
        );
        assert_eq!(
            space.move_by(bfvec(2.into(), 2.into()), bfvec((-3).into(), (-3).into())),
            (bfvec(2.into(), 2.into()), &T::from('a' as i32))
        );
        assert_eq!(
            space.move_by(bfvec(0.into(), 0.into()), bfvec((-2).into(), 0.into())),
            (bfvec(4.into(), 0.into()), &T::from('5' as i32))
        );
        assert_eq!(
            space.move_by(bfvec(4.into(), 0.into()), bfvec(0.into(), 1.into())),
            (bfvec(4.into(), 2.into()), &T::from('b' as i32))
        );
        assert_eq!(
            space.move_by(bfvec(7.into(), 0.into()), bfvec(2.into(), (-1).into())),
            (bfvec(1.into(), 3.into()), &T::from('A' as i32))
        );

        // Try something very far away
        space[bfvec(32000.into(), 8000.into())] = T::from('0' as i32);
        space[bfvec(32000.into(), 2.into())] = T::from('0' as i32);
        assert_eq!(
            space.move_by(bfvec(0.into(), 0.into()), bfvec(4.into(), 1.into())),
            (bfvec(32000.into(), 8000.into()), &T::from('0' as i32))
        );
        assert_eq!(
            space.move_by(bfvec(32000.into(), 8000.into()), bfvec(0.into(), 1.into())),
            (bfvec(32000.into(), 2.into()), &T::from('0' as i32))
        );
        assert_eq!(
            space.move_by(bfvec(32000.into(), 2.into()), bfvec((-1).into(), 0.into())),
            (bfvec(9.into(), 2.into()), &T::from('c' as i32))
        );

        // Check the dimensions
        assert_eq!(space.min_idx(), Some(bfvec(0.into(), 0.into())));
        assert_eq!(space.max_idx(), Some(bfvec(32000.into(), 8000.into())));
    }
}
