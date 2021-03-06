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

use std::cmp::{max, min};
use std::fmt::{Display, Formatter};
use std::ops::{Add, Div, Mul, Rem, Sub};

use divrem::{DivEuclid, DivRem, DivRemEuclid, RemEuclid};

use super::{FungeArrayIdx, FungeIndex, FungeValue};

// ----------------------------------------------------------------------
// Implementation of funge index traits for scalars (Unefunge)
// ----------------------------------------------------------------------

impl<T> FungeIndex for T
where
    T: FungeValue,
{
    const RANK: i32 = 1;

    fn joint_min(&self, other: &Self) -> Self {
        min(*self, *other)
    }

    fn joint_max(&self, other: &Self) -> Self {
        max(*self, *other)
    }

    fn find_joint_min_where<Pred>(
        pred: &mut Pred,
        absolute_min: &Self,
        absolute_max: &Self,
    ) -> Option<Self>
    where
        Pred: FnMut(&Self) -> bool,
    {
        let mut i = *absolute_min;
        while i < *absolute_max {
            if pred(&i) {
                return Some(i);
            }
            i += 1.into()
        }
        None
    }

    fn find_joint_max_where<Pred>(
        pred: &mut Pred,
        absolute_min: &Self,
        absolute_max: &Self,
    ) -> Option<Self>
    where
        Pred: FnMut(&Self) -> bool,
    {
        let mut i = *absolute_max - 1.into();
        while i >= *absolute_min {
            if pred(&i) {
                return Some(i);
            }
            i -= 1.into()
        }
        None
    }

    fn origin() -> Self {
        0.into()
    }
}

impl<T> FungeArrayIdx for T
where
    T: FungeValue + RemEuclid,
{
    fn to_lin_index(&self, array_size: &Self) -> usize {
        self.rem_euclid(*array_size).to_usize().unwrap()
    }

    fn to_lin_index_unchecked(&self, _array_size: &Self) -> usize {
        self.to_usize().unwrap()
    }

    fn from_lin_index(lin_idx: usize, _array_size: &Self) -> Self {
        T::from_usize(lin_idx).unwrap()
    }

    fn lin_size(&self) -> usize {
        self.to_usize().unwrap()
    }
}

// ----------------------------------------------------------------------
// Befunge / 2D index type
// ----------------------------------------------------------------------

/// A Befunge index
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BefungeVec<T>
where
    T: FungeValue,
{
    pub x: T,
    pub y: T,
}

/// Convenience function to create a [BefungeVec]
pub fn bfvec<Tout, Tin>(x: Tin, y: Tin) -> BefungeVec<Tout>
where
    Tout: FungeValue,
    Tin: Into<Tout>,
{
    BefungeVec::<Tout> {
        x: x.into(),
        y: y.into(),
    }
}

impl<T> Display for BefungeVec<T>
where
    T: FungeValue,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl<T> Add for BefungeVec<T>
where
    T: FungeValue,
{
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl<T> Sub for BefungeVec<T>
where
    T: FungeValue,
{
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl<T> Mul<T> for BefungeVec<T>
where
    T: FungeValue,
{
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: T) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl<T> Mul for BefungeVec<T>
where
    T: FungeValue,
{
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: Self) -> Self {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
        }
    }
}

impl<T> Div for BefungeVec<T>
where
    T: FungeValue,
{
    type Output = Self;
    #[inline(always)]
    fn div(self, rhs: Self) -> Self {
        Self {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
        }
    }
}

impl<T> Rem for BefungeVec<T>
where
    T: FungeValue,
{
    type Output = Self;
    #[inline(always)]
    fn rem(self, rhs: Self) -> Self {
        Self {
            x: self.x % rhs.x,
            y: self.y % rhs.y,
        }
    }
}

impl<T> DivRem for BefungeVec<T>
where
    T: FungeValue,
{
    type Output = (Self, Self);
    #[inline(always)]
    fn div_rem(self, rhs: Self) -> (Self, Self) {
        let (x_d, x_r) = self.x.div_rem(rhs.x);
        let (y_d, y_r) = self.y.div_rem(rhs.y);
        (Self { x: x_d, y: y_d }, Self { x: x_r, y: y_r })
    }
}

impl<T> DivEuclid for BefungeVec<T>
where
    T: FungeValue + DivEuclid,
{
    #[inline(always)]
    fn div_euclid(self, rhs: Self) -> Self {
        Self {
            x: self.x.div_euclid(rhs.x),
            y: self.y.div_euclid(rhs.y),
        }
    }
}

impl<T> RemEuclid for BefungeVec<T>
where
    T: FungeValue + RemEuclid,
{
    #[inline(always)]
    fn rem_euclid(self, rhs: Self) -> Self {
        Self {
            x: self.x.rem_euclid(rhs.x),
            y: self.y.rem_euclid(rhs.y),
        }
    }
}

impl<T> DivRemEuclid for BefungeVec<T>
where
    T: FungeValue + DivRemEuclid,
{
    #[inline(always)]
    fn div_rem_euclid(self, rhs: Self) -> (Self, Self) {
        let (x_d, x_r) = self.x.div_rem_euclid(rhs.x);
        let (y_d, y_r) = self.y.div_rem_euclid(rhs.y);
        (Self { x: x_d, y: y_d }, Self { x: x_r, y: y_r })
    }
}

impl<T> FungeIndex for BefungeVec<T>
where
    T: FungeValue,
{
    const RANK: i32 = 1;

    #[inline(always)]
    fn joint_min(&self, other: &Self) -> Self {
        Self {
            x: min(self.x, other.x),
            y: min(self.y, other.y),
        }
    }

    #[inline(always)]
    fn joint_max(&self, other: &Self) -> Self {
        Self {
            x: max(self.x, other.x),
            y: max(self.y, other.y),
        }
    }

    fn find_joint_min_where<Pred>(
        pred: &mut Pred,
        absolute_min: &Self,
        absolute_max: &Self,
    ) -> Option<Self>
    where
        Pred: FnMut(&Self) -> bool,
    {
        let mut hypothesis = *absolute_min;

        'outer: while hypothesis.x < absolute_max.x && hypothesis.y < absolute_max.y {
            let mut min_x = hypothesis.x;
            while !pred(&Self {
                x: min_x,
                y: hypothesis.y,
            }) {
                min_x += 1.into();
                if min_x >= absolute_max.x {
                    // move down one row
                    hypothesis.y += 1.into();
                    continue 'outer;
                }
            }
            let mut min_y = hypothesis.y;
            while !pred(&Self {
                x: hypothesis.x,
                y: min_y,
            }) {
                min_y += 1.into();
                if min_y >= absolute_max.y {
                    // move across one column
                    hypothesis.x += 1.into();
                    continue 'outer;
                }
            }
            // We only get this far if there is a valid value for our x and y
            return Some(hypothesis);
        }
        // No valid values
        None
    }

    fn find_joint_max_where<Pred>(
        pred: &mut Pred,
        absolute_min: &Self,
        absolute_max: &Self,
    ) -> Option<Self>
    where
        Pred: FnMut(&Self) -> bool,
    {
        let mut hypothesis = Self {
            x: absolute_max.x - 1.into(),
            y: absolute_max.y - 1.into(),
        };

        'outer: while hypothesis.x >= absolute_min.x && hypothesis.y >= absolute_min.y {
            let mut max_x = hypothesis.x;
            while !pred(&Self {
                x: max_x,
                y: hypothesis.y,
            }) {
                max_x -= 1.into();
                if max_x < absolute_min.x {
                    // move up one row
                    hypothesis.y -= 1.into();
                    continue 'outer;
                }
            }
            let mut max_y = hypothesis.y;
            while !pred(&Self {
                x: hypothesis.x,
                y: max_y,
            }) {
                max_y -= 1.into();
                if max_y < absolute_min.y {
                    // move across one column
                    hypothesis.x -= 1.into();
                    continue 'outer;
                }
            }
            // We only get this far if there is a valid value for our x and y
            return Some(hypothesis);
        }
        // No valid values
        None
    }

    fn origin() -> Self {
        bfvec(0, 0)
    }
}

impl<T> FungeArrayIdx for BefungeVec<T>
where
    T: FungeValue + RemEuclid,
{
    fn to_lin_index(&self, array_size: &Self) -> usize {
        let trunc = self.rem_euclid(*array_size);
        (trunc.x + trunc.y * array_size.x).to_usize().unwrap()
    }

    fn to_lin_index_unchecked(&self, array_size: &Self) -> usize {
        (self.x + self.y * array_size.x).to_i64().unwrap() as usize
    }

    fn from_lin_index(lin_idx: usize, array_size: &Self) -> Self {
        let width: T = array_size.x.to_i32().unwrap().into();
        let (y, x) = T::from(lin_idx as i32).div_rem(width);
        Self { x, y }
    }

    fn lin_size(&self) -> usize {
        (self.x * self.y).to_usize().unwrap()
    }
}

// ----------------------------------------------------------------------
// TESTS
// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1d_min_max() {
        let a: i64 = -1;
        let b: i64 = 7;
        let c: i64 = -129;
        let d: i64 = 1093;
        assert_eq!(a.joint_min(&c), c);
        assert_eq!(b.joint_max(&c), b);
        assert_eq!(c.joint_min(&d), c);
        assert_eq!(a.joint_max(&d), d);
    }

    #[test]
    fn test_1d_arraymethods() {
        assert_eq!((100 as i64).to_lin_index(&200), 100);
        assert_eq!((100 as i64).to_lin_index(&90), 10);
        assert_eq!((-3 as i64).to_lin_index(&100), 97);
        assert_eq!(<i64 as FungeArrayIdx>::from_lin_index(76, &100), 76);
        assert_eq!((874 as i64).lin_size(), 874);
    }

    #[test]
    fn test_2d_math() {
        assert_eq!(bfvec(0, 5) + bfvec(12, -3), bfvec::<i32, _>(12, 2));
        assert_eq!(bfvec(3, 4) - bfvec(7, 15), bfvec::<i32, _>(-4, -11));
        assert_eq!(bfvec(4, 7) * 3, bfvec(12, 21));
        assert_eq!(bfvec(-32, -27) / bfvec(16, 16), bfvec::<i32, _>(-2, -1));
        assert_eq!(
            bfvec(-32, -27).div_euclid(bfvec(16, 16)),
            bfvec::<i32, _>(-2, -2)
        );
        assert_eq!(
            bfvec::<i32, _>(56, -3).div_rem_euclid(bfvec(-25, -25)),
            (bfvec(-2, 1), bfvec(6, 22))
        );
    }

    #[test]
    fn test_2d_min_max() {
        assert_eq!(bfvec::<i32, _>(0, 5).joint_min(&bfvec(2, 2)), bfvec(0, 2));
        assert_eq!(
            bfvec::<i32, _>(9, 12).joint_max(&bfvec(10, 5)),
            bfvec(10, 12)
        );
    }

    #[test]
    fn test_2d_arraymethods() {
        assert_eq!(bfvec::<i32, _>(5, 3).to_lin_index(&bfvec(10, 10)), 35);
        assert_eq!(
            BefungeVec::<i32>::from_lin_index(13, &bfvec(6, 10)),
            bfvec(1, 2)
        );
        assert_eq!(bfvec::<i32, _>(13, 5).lin_size(), 65);
    }
}
