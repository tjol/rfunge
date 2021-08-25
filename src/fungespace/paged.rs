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

use std::cmp::Ordering;
use std::hash::Hash;
use std::ops::{Add, Div, Index, IndexMut, Mul, Rem};

use divrem::{DivEuclid, DivRem, DivRemEuclid, RemEuclid};
use hashbrown::HashMap;
use num::{One, Zero};

use super::index::{bfvec, BefungeVec};
use super::{FungeArrayIdx, FungeSpace, FungeValue};

/// Trait required for indices when used with [PagedFungeSpace]
pub trait PageSpaceVector<T>:
    Mul<T, Output = Self>
    + FungeArrayIdx
    + Div<Output = Self>
    + Rem<Output = Self>
    + DivEuclid
    + DivRem<Output = (Self, Self)>
    + DivRemEuclid
    + RemEuclid
    + Add<Output = Self>
    + Mul<Output = Self>
    + Hash
where
    T: FungeValue,
{
    /// Return `Some(n)`, where `n` is the smallest integer such that
    /// `self + n * delta` lies within the line segment (unefunge),
    /// rectangle (befunge) or cuboid (trefunge) spannig from `start`
    /// (inclusive) to `start + size` (exclusive).
    ///
    /// If there is no such point (becuase the line defined `self + n * delta`
    /// doesn't pass through the region indicated), return `None`.
    fn dist_of_region(&self, delta: &Self, start: &Self, size: &Self) -> Option<T>;

    /// Call a closure for every idx = start + n * delta, n = 0, 1, ...,
    /// such that idx lies within a region of size `limit` starting at the
    /// origin, until the closure returns `true`. Returns `true` if the loop
    /// was stopped because the closure returned `true`, return `false` if
    /// it always returned `false`.
    fn scan_within_region<F>(start: &Self, delta: &Self, limit: &Self, callback: &mut F) -> bool
    where
        F: FnMut(&Self) -> bool;
}

/// Implementation of funge space that stores fixed-size segments of funge-space
/// as arrays.
pub struct PagedFungeSpace<Idx, Elem>
where
    Idx: PageSpaceVector<Elem>,
    Elem: FungeValue,
{
    page_size: Idx,
    pages: HashMap<Idx, Vec<Elem>>,
    _blank: Elem, // This should really be const but I don't know how to do that
}

impl<Idx, Elem> PagedFungeSpace<Idx, Elem>
where
    Idx: PageSpaceVector<Elem>,
    Elem: FungeValue,
{
    pub fn new_with_page_size(page_size: Idx) -> Self {
        Self {
            page_size,
            pages: HashMap::new(),
            _blank: Elem::from(' ' as i32),
        }
    }
}

impl<Idx, Elem> Index<Idx> for PagedFungeSpace<Idx, Elem>
where
    Idx: PageSpaceVector<Elem>,
    Elem: FungeValue,
{
    type Output = Elem;
    fn index(&self, idx: Idx) -> &Elem {
        let (page_idx, idx_in_page) = idx.div_rem_euclid(self.page_size);
        if let Some(page) = self.pages.get(&page_idx) {
            &page[idx_in_page.to_lin_index(&self.page_size)]
        } else {
            &self._blank
        }
    }
}

impl<Idx, Elem> IndexMut<Idx> for PagedFungeSpace<Idx, Elem>
where
    Idx: PageSpaceVector<Elem>,
    Elem: FungeValue,
{
    fn index_mut(&mut self, idx: Idx) -> &mut Elem {
        let (page_idx, idx_in_page) = idx.div_rem_euclid(self.page_size);
        if !self.pages.contains_key(&page_idx) {
            let mut v = Vec::new();
            v.resize(self.page_size.lin_size(), self._blank);
            self.pages.insert(page_idx, v);
        }
        let page = self.pages.get_mut(&page_idx).unwrap();
        let lin_idx = idx_in_page.to_lin_index(&self.page_size);
        page.index_mut(lin_idx)
    }
}

impl<Idx, Elem> FungeSpace<Idx> for PagedFungeSpace<Idx, Elem>
where
    Idx: PageSpaceVector<Elem>,
    Elem: FungeValue,
{
    fn move_by(&self, start: Idx, delta: Idx) -> (Idx, &Elem) {
        let mut idx = start + delta;
        let (mut page_idx, mut idx_in_page) = idx.div_rem_euclid(self.page_size);

        // first, lets try a straight scan
        while let Some(this_page) = self.pages.get(&page_idx) {
            let mut the_value = &self._blank;
            let mut the_idx = idx;
            let mut last_idx_in_page = idx_in_page;
            let mut scan_closure = |this_idx: &Idx| {
                last_idx_in_page = *this_idx;
                let lin_idx = this_idx.to_lin_index_unchecked(&self.page_size);
                let v = &this_page[lin_idx];
                if *v != self._blank {
                    the_value = v;
                    the_idx = page_idx * self.page_size + *this_idx;
                    true
                } else {
                    false
                }
            };
            if Idx::scan_within_region(&idx_in_page, &delta, &self.page_size, &mut scan_closure) {
                return (the_idx, the_value);
            } else {
                // Not found, move on
                idx = page_idx * self.page_size + last_idx_in_page + delta;
                let (q, r) = idx.div_rem_euclid(self.page_size);
                page_idx = q;
                idx_in_page = r;
            }
        }

        // We've hit the edge, time for some maths
        let cur_dist = idx
            .dist_of_region(&delta, &(page_idx * self.page_size), &self.page_size)
            .unwrap();

        let mut page_dists: Vec<(Idx, Elem)> = self
            .pages
            .keys()
            .filter_map(|k| {
                Some((
                    *k,
                    start.dist_of_region(&delta, &(*k * self.page_size), &self.page_size)?,
                ))
            })
            .filter(|(_, d)| *d > cur_dist || *d <= Zero::zero())
            .collect();
        page_dists.sort_by_key(|(_, d)| (*d <= Zero::zero(), *d));

        for (target_page_idx, dist) in page_dists.into_iter() {
            idx = start + delta * dist;
            page_idx = target_page_idx;
            idx_in_page = idx.rem_euclid(self.page_size);
            while page_idx == target_page_idx {
                let this_page = &self.pages[&page_idx];
                let mut the_value = &self._blank;
                let mut the_idx = idx;
                let mut last_idx_in_page = idx_in_page;
                let mut scan_closure = |this_idx: &Idx| {
                    last_idx_in_page = *this_idx;
                    let lin_idx = this_idx.to_lin_index_unchecked(&self.page_size);
                    let v = &this_page[lin_idx];
                    if *v != self._blank {
                        the_value = v;
                        the_idx = page_idx * self.page_size + *this_idx;
                        true
                    } else {
                        false
                    }
                };
                if Idx::scan_within_region(&idx_in_page, &delta, &self.page_size, &mut scan_closure)
                {
                    return (the_idx, the_value);
                } else {
                    // Not found, move on
                    idx = page_idx * self.page_size + last_idx_in_page + delta;
                    let (q, r) = idx.div_rem_euclid(self.page_size);
                    page_idx = q;
                    idx_in_page = r;
                }
            }
        }

        // NOTHING found? This is a problem, but probably the IP's
        (start, &self[start])
    }

    fn min_idx(&self) -> Option<Idx> {
        self.pages
            .iter()
            .filter_map(|(k, p)| {
                Idx::find_joint_min_where(
                    &mut |idx: &Idx| p[idx.to_lin_index(&self.page_size)] != (' ' as i32).into(),
                    &Idx::origin(),
                    &self.page_size,
                )
                .map(|min_idx| min_idx + (*k * self.page_size))
            })
            .reduce(|i1, i2| i1.joint_min(&i2))
    }

    fn max_idx(&self) -> Option<Idx> {
        self.pages
            .iter()
            .filter_map(|(k, p)| {
                Idx::find_joint_max_where(
                    &mut |idx: &Idx| p[idx.to_lin_index(&self.page_size)] != (' ' as i32).into(),
                    &Idx::origin(),
                    &self.page_size,
                )
                .map(|max_idx| max_idx + (*k * self.page_size))
            })
            .reduce(|i1, i2| i1.joint_max(&i2))
    }
}

impl<T> PageSpaceVector<T> for T
where
    T: FungeValue + Hash + DivEuclid + RemEuclid + DivRemEuclid,
{
    fn dist_of_region(&self, delta: &Self, start: &Self, size: &Self) -> Option<T> {
        match (*delta).cmp(&Zero::zero()) {
            Ordering::Greater => {
                // going forward
                let (dist, rem) = (*start - *self).div_rem_euclid(*delta);

                if Zero::is_zero(&rem) {
                    Some(dist)
                } else if (*self) + (dist + One::one()) * (*delta) < ((*start) + (*size)) {
                    Some(dist + One::one())
                } else {
                    None
                }
            }
            Ordering::Equal => None,
            Ordering::Less => {
                // going backward
                let dist = ((*start) + (*size) - One::one() - (*self)).div_euclid(*delta);

                if (*self) + dist * (*delta) >= (*start) {
                    Some(dist)
                } else {
                    None
                }
            }
        }
    }

    fn scan_within_region<F>(start: &Self, delta: &Self, limit: &Self, callback: &mut F) -> bool
    where
        F: FnMut(&Self) -> bool,
    {
        let mut idx = *start;
        while idx >= Zero::zero() && idx < *limit {
            if callback(&idx) {
                return true;
            }
            idx += *delta;
        }
        false
    }
}

impl<T> PageSpaceVector<T> for BefungeVec<T>
where
    T: FungeValue + RemEuclid + Hash + DivEuclid + DivRemEuclid,
{
    fn dist_of_region(&self, delta: &Self, start: &Self, size: &Self) -> Option<T> {
        // If the top-left corner and the bottom-right corner of the region
        // are on opposite sides of the line, we might have a hit.
        let rel_topleft = *start - *self;
        let rel_bottomright = (*start + *size) - *self;
        let rel_topright = bfvec::<T, T>(rel_bottomright.x, rel_topleft.y);
        let rel_bottomleft = bfvec::<T, T>(rel_topleft.x, rel_bottomright.y);
        let cross_tl = rel_topleft.x * delta.y - delta.x * rel_topleft.y;
        let cross_br = rel_bottomright.x * delta.y - delta.x * rel_bottomright.y;
        let cross_tr = rel_topright.x * delta.y - delta.x * rel_topright.y;
        let cross_bl = rel_bottomleft.x * delta.y - delta.x * rel_bottomleft.y;
        if cross_tl.signum() != cross_br.signum() || cross_tr.signum() != cross_bl.signum() {
            // The line crosses our region. Is there a "stop"?
            if Zero::is_zero(&delta.x) {
                self.y.dist_of_region(&delta.y, &start.y, &size.y)
            } else {
                let mut dist = self.x.dist_of_region(&delta.x, &start.x, &size.x)?;
                let mut first_pos = *self + *delta * dist;

                // Make sure Y in in bounds
                while first_pos.y < start.y || first_pos.y >= start.y + size.y {
                    dist += One::one();
                    first_pos = *self + *delta * dist;
                    if first_pos.x >= start.x + size.x {
                        // Oops, we overshot
                        return None;
                    }
                }

                Some(dist)
            }
        } else {
            None
        }
    }

    fn scan_within_region<F>(start: &Self, delta: &Self, limit: &Self, callback: &mut F) -> bool
    where
        F: FnMut(&Self) -> bool,
    {
        let mut idx = *start;
        while idx.x >= Zero::zero() && idx.x < limit.x && idx.y >= Zero::zero() && idx.y < limit.y {
            if callback(&idx) {
                return true;
            }
            idx = idx + *delta;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::super::index::{bfvec, BefungeVec};
    use super::super::tests as gen_tests;
    use super::*;

    #[test]
    fn test_unefunge_motion() {
        let mut space = PagedFungeSpace::<i64, i64>::new_with_page_size(128);
        gen_tests::test_unefunge_motion(&mut space);
    }

    #[test]
    fn test_befunge_motion() {
        let mut space = PagedFungeSpace::<BefungeVec<i64>, i64>::new_with_page_size(bfvec(80, 25));
        gen_tests::test_befunge_motion(&mut space);
    }
}
