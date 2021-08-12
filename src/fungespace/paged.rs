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

use std::collections::HashMap;
use std::hash::Hash;
use std::ops::{Add, Div, Index, IndexMut, Mul};

use divrem::{DivEuclid, DivRem, DivRemEuclid, RemEuclid};

use super::index::{bfvec, BefungeVec};
use super::{FungeArrayIdx, FungeSpace, FungeValue};

/// Trait required for indices when used with [PagedFungeSpace]
pub trait PageSpaceVector<T>:
    Mul<T, Output = Self>
    + FungeArrayIdx
    + Div<Output = Self>
    + DivEuclid
    + DivRem<Output = (Self, Self)>
    + DivRemEuclid
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
            page_size: page_size,
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
        return page.index_mut(lin_idx);
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
        while self.pages.contains_key(&page_idx) {
            let lin_idx = idx_in_page.to_lin_index(&self.page_size);
            let v = self.pages.get(&page_idx).unwrap().index(lin_idx);
            if *v != self._blank {
                return (idx, v);
            }
            idx = idx + delta;
            let (q, r) = idx.div_rem_euclid(self.page_size);
            page_idx = q;
            idx_in_page = r;
        }

        // We've hit the edge, time for some maths
        let mut page_dists: Vec<(Idx, Elem)> = self
            .pages
            .keys()
            .filter_map(|k| {
                Some((
                    *k,
                    start.dist_of_region(&delta, &(*k * self.page_size), &self.page_size)?,
                ))
            })
            .collect();
        page_dists.sort_by_key(|(_, d)| *d);

        // let cur_page = start.div_euclid(self.page_size);
        // let cur_dist = start
        //     .dist_of_region(&delta, &(cur_page * self.page_size), &self.page_size)
        //     .unwrap();
        let cur_dist: Elem = 0.into();

        // Are there any pages further than the one we last checked?
        let pages_ahead = page_dists.iter().filter(|(_, d)| *d > cur_dist);
        // Are there any further back? -- there must be
        let pages_astern = page_dists.iter().filter(|(_, d)| *d <= 0.into());
        let distant_pages = pages_ahead.chain(pages_astern);

        for (target_page_idx, dist) in distant_pages {
            let mut new_idx = start + delta * (*dist);
            let (mut cur_page_idx, mut idx_in_page) = new_idx.div_rem_euclid(self.page_size);
            while cur_page_idx == *target_page_idx {
                let lin_idx = idx_in_page.to_lin_index(&self.page_size);
                let v = self.pages.get(target_page_idx).unwrap().index(lin_idx);
                if *v != self._blank {
                    return (new_idx, v);
                }
                new_idx = new_idx + delta;
                let (q, r) = new_idx.div_rem_euclid(self.page_size);
                cur_page_idx = q;
                idx_in_page = r;
            }
        }

        // NOTHING found? This is a problem, but probably the IP's
        return (start, &self[start]);
    }

    fn min_idx(&self) -> Option<Idx> {
        self.pages
            .iter()
            .map(|(k, p)| {
                *k * self.page_size
                    + (0..p.len())
                        .filter(|i| self.pages.get(k).unwrap()[*i] != self._blank)
                        .map(|i| Idx::from_lin_index(i, &self.page_size))
                        .reduce(|i1, i2| i1.joint_min(&i2))
                        .unwrap()
            })
            .reduce(|i1, i2| i1.joint_min(&i2))
    }

    fn max_idx(&self) -> Option<Idx> {
        self.pages
            .iter()
            .map(|(k, p)| {
                *k * self.page_size
                    + (0..p.len())
                        .filter(|i| self.pages.get(k).unwrap()[*i] != self._blank)
                        .map(|i| Idx::from_lin_index(i, &self.page_size))
                        .reduce(|i1, i2| i1.joint_max(&i2))
                        .unwrap()
            })
            .reduce(|i1, i2| i1.joint_max(&i2))
    }
}

impl<T> PageSpaceVector<T> for T
where
    T: FungeValue + Hash + DivEuclid + RemEuclid + DivRemEuclid,
{
    fn dist_of_region(&self, delta: &Self, start: &Self, size: &Self) -> Option<T> {
        if *delta > 0.into() {
            let (dist, rem) = (*start - *self).div_rem_euclid(*delta);

            if rem == 0.into() {
                Some(dist)
            } else if (*self) + (dist + 1.into()) * (*delta) < ((*start) + (*size)) {
                Some(dist + 1.into())
            } else {
                None
            }
        } else if *delta < 0.into() {
            let dist = ((*start) + (*size) - 1.into() - (*self)).div_euclid(*delta);

            if (*self) + dist * (*delta) >= (*start) {
                Some(dist)
            } else {
                None
            }
        } else {
            None
        }
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
            if delta.x == 0.into() {
                return self.y.dist_of_region(&delta.y, &start.y, &size.y);
            } else {
                let mut dist = self.x.dist_of_region(&delta.x, &start.x, &size.x)?;
                let mut first_pos = *self + *delta * dist;

                // Make sure Y in in bounds
                while first_pos.y < start.y || first_pos.y >= start.y + size.y {
                    dist += 1.into();
                    first_pos = *self + *delta * dist;
                    if first_pos.x >= start.x + size.x {
                        // Oops, we overshot
                        return None;
                    }
                }

                return Some(dist);
            }
        } else {
            return None;
        }
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
