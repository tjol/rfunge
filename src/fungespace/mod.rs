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

use std::cmp::max;
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
pub trait FungeIndex: Eq + Copy + Debug + 'static {
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

    /// Get the minimum index (as defined for [Self::joint_min]), no lower than
    /// `absolute_min`, and lower than `absolute_max` (on all components) for
    /// which the predicate `pred` returns true
    ///
    /// This is used, e.g., to determine the maximum extent of funge-space.
    fn find_joint_min_where<Pred>(
        pred: &mut Pred,
        absolute_min: &Self,
        absolute_max: &Self,
    ) -> Option<Self>
    where
        Pred: FnMut(&Self) -> bool;

    /// Get the maximum index (as defined for [Self::joint_max]), no lower than
    /// `absolute_min`, and lower than `absolute_max` (on all components) for
    /// which the predicate `pred` returns true
    ///
    /// This is used, e.g., to determine the maximum extent of funge-space.
    fn find_joint_max_where<Pred>(
        pred: &mut Pred,
        absolute_min: &Self,
        absolute_max: &Self,
    ) -> Option<Self>
    where
        Pred: FnMut(&Self) -> bool;

    /// The number of scalars per vector
    fn rank() -> i32;

    /// Get the index corresponding to the origin
    fn origin() -> Self;
}

/// Generic trait representing a theoretically infinite funge-space, and
/// implementing Lahey-space wrapping.
pub trait FungeSpace<Idx>: Index<Idx> + IndexMut<Idx>
where
    Idx: FungeIndex,
{
    /// Move by `delta`, starting from `start`, according to Funge-98 rules.
    ///
    /// Stops at the next non-space character, and returns a tuple of the
    /// index of the new position and (a reference to) the value found there.
    ///
    /// Does not skip over `;` cells
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

/// A value that can live in funge space (automatically implemented for any
/// type that implements the prerequisites, in particular `i32` and `i64`)
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
    + 'static
{
    /// Return the value as a character, if the unicode code point exists
    fn try_to_char(&self) -> Option<char> {
        self.to_u32().and_then(char::from_u32)
    }
    /// Return the value as a character, or U+FFFD �
    fn to_char(&self) -> char {
        self.try_to_char().unwrap_or('�')
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
        + 'static
{
}

/// Trait for reading and writing the contents of a [FungeSpace] as funge-98
/// source code.
pub trait SrcIO<Space>: FungeIndex
where
    Space: FungeSpace<Self>,
    Space::Output: FungeValue,
{
    /// Read a binary/latin1 file (`src`) into `space` starting at index
    /// `start`; returns the size of the region written to.
    fn read_bin_at(space: &mut Space, start: &Self, src: &[u8]) -> Self;

    /// Read a unicode file (`src`) into `space` starting at index
    /// `start`; returns the size of the region written to.
    fn read_str_at(space: &mut Space, start: &Self, src: &str) -> Self;

    /// Get the region of `space` starting at `start` with size `size` as
    /// funge-98 source code, independently of encoding. If `strip` is `true`,
    /// trailing spaces/newlines/etc should be removed.
    fn get_src_region(space: &Space, start: &Self, size: &Self, strip: bool) -> Vec<Space::Output>;

    /// Like [SrcIO::get_src_region], but returns a UTF-8 string (replacing
    /// out-of-range values with U+FFFD �)
    fn get_src_str(space: &Space, start: &Self, size: &Self, strip: bool) -> String {
        Self::get_src_region(space, start, size, strip)
            .into_iter()
            .map(|v| v.to_char())
            .collect()
    }

    /// Like [SrcIO::get_src_region], but returns a byte string (consisting of
    /// the least significant 8 bits of each value only)
    fn get_src_bin(space: &Space, start: &Self, size: &Self, strip: bool) -> Vec<u8> {
        Self::get_src_region(space, start, size, strip)
            .into_iter()
            .map(|v| v.to_u8().unwrap_or(0xff))
            .collect()
    }
}

/// SrcIO implementation for unefunge
impl<Space, T> SrcIO<Space> for T
where
    T: FungeValue + FungeIndex,
    Space: FungeSpace<T> + Index<T, Output = T>,
{
    /// Read a binary / latin1 file into a unefunge space starting at position `start`
    fn read_bin_at(space: &mut Space, start: &Self, src: &[u8]) -> Self {
        let mut idx = *start;

        for byte in src {
            match byte {
                10 | 12 | 13 => {} // skip CR & FF & LF
                byte => {
                    let value = *byte as i32;
                    if value != (' ' as i32) {
                        space[idx] = value.into();
                    }
                    idx += 1.into();
                }
            }
        }

        idx - *start
    }

    /// Read a string into unifunge space starting at position `start`
    fn read_str_at(space: &mut Space, start: &Self, src: &str) -> Self {
        let mut i = *start;

        for line in src.lines() {
            for c in line.chars() {
                if c != '\x0c' {
                    if c != ' ' {
                        space[i] = (c as i32).into();
                    }
                    i += 1.into();
                }
            }
        }

        i - *start
    }

    fn get_src_region(space: &Space, start: &Self, size: &Self, strip: bool) -> Vec<Space::Output> {
        let mut src = Vec::new();
        if *size < 0.into() {
            return src;
        }
        src.reserve_exact(size.to_usize().unwrap());
        for i in 0..size.to_i32().unwrap() {
            src[i as usize] = space[Self::from(i) + *start];
        }
        if strip {
            while src[src.len() - 1] == T::from(' ' as i32) {
                src.pop();
            }
        }
        src
    }
}

/// SrcIO implementation for befunge
impl<Space, T> SrcIO<Space> for BefungeVec<T>
where
    T: FungeValue,
    Space: FungeSpace<BefungeVec<T>> + Index<BefungeVec<T>, Output = T>,
{
    /// Read a binary / latin1 file into a unefunge space starting at position `start`
    fn read_bin_at(space: &mut Space, start: &Self, src: &[u8]) -> Self {
        let mut x: T = start.x;
        let mut y: T = start.y;
        let mut max_x: T = start.x;
        let mut recent_cr = false;
        for byte in src {
            match byte {
                10 => {
                    // line feed
                    if !recent_cr {
                        max_x = max(x, max_x);
                        x = start.x;
                        y += 1.into();
                    }
                    recent_cr = false;
                }
                13 => {
                    // carriage return
                    max_x = max(x, max_x);
                    x = start.x;
                    y += 1.into();
                    recent_cr = true;
                }
                12 => {
                    // form feed
                    // do nothing
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
        max_x = max(x, max_x);
        if x != start.x {
            y += 1.into();
        }
        Self { x: max_x, y } - *start
    }

    /// Read a string into unifunge space starting at position `start`
    fn read_str_at(space: &mut Space, start: &Self, src: &str) -> Self {
        let mut max_x: T = 0.into();
        let mut max_y: T = 0.into();
        for (y, line) in src.lines().enumerate() {
            for (x, c) in line.chars().enumerate() {
                if c != '\x0c' {
                    if c != ' ' {
                        space[*start
                            + bfvec(T::from_usize(x).unwrap(), T::from_usize(y).unwrap())] =
                            (c as i32).into();
                    }
                    max_x = max(((x + 1) as i32).into(), max_x);
                }
            }
            max_y = max(((y + 1) as i32).into(), max_y);
        }
        Self { x: max_x, y: max_y }
    }

    fn get_src_region(space: &Space, start: &Self, size: &Self, strip: bool) -> Vec<Space::Output> {
        if size.x < 0.into() || size.y < 0.into() {
            return Vec::new();
        }

        let mut src = Vec::new();
        let size_x = size.x.to_usize().unwrap();
        let size_y = size.y.to_usize().unwrap();

        for y_out in 0..size_y {
            if y_out != 0 {
                src.push(('\n' as i32).into());
            }
            let y_in = T::from_usize(y_out).unwrap() + start.y;
            let mut n_spaces = 0;
            for x_out in 0..size_x {
                let x_in = T::from_usize(x_out).unwrap() + start.x;
                let val = space[Self { x: x_in, y: y_in }];
                if val == (' ' as i32).into() {
                    // Skip spaces at the end
                    n_spaces += 1;
                } else {
                    // Put spaces back
                    for _ in 0..n_spaces {
                        src.push((' ' as i32).into());
                    }
                    n_spaces = 0;
                    src.push(val);
                }
            }
            if !strip {
                for _ in 0..n_spaces {
                    src.push((' ' as i32).into());
                }
            }
        }

        if strip {
            while !src.is_empty() && src[src.len() - 1] == ('\n' as i32).into() {
                src.pop();
            }
        }

        src
    }
}

/// Read a string into a funge space
pub fn read_funge_src<Idx, Space>(space: &mut Space, src: &str) -> Idx
where
    Space: FungeSpace<Idx>,
    Idx: SrcIO<Space>,
    Space::Output: FungeValue,
{
    Idx::read_str_at(space, &Idx::origin(), src)
}

/// Read a binary/latin-1 buffer into a funge space
pub fn read_funge_src_bin<Idx, Space>(space: &mut Space, src: &[u8]) -> Idx
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
