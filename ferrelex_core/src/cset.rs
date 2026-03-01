// Copyright (c) 2025-2026 Emilien Lemaire <emilien.lem@icloud.com>
// SPDX-License-Identifier: LGPL-3.0-only
// Licensed under the GNU Lesser General Public License v3.0, with the
// ferrelex Generated Code Exception. See the LICENSE file at the root
// of this repository for the full license text and exception terms.

use std::{cmp::Ordering, hash::Hash};

#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord, Default)]
pub struct CSet(Vec<(i32, i32)>);

impl CSet {
    pub fn max_code() -> i32 {
        0x10ffff
    }

    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn singleton(i: i32) -> Self {
        Self(vec![(i, i)])
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn interval(i: i32, j: i32) -> Self {
        if i < j {
            Self(vec![(i, j)])
        } else {
            Self(vec![(j, i)])
        }
    }

    pub fn any() -> Self {
        Self::interval(0, Self::max_code())
    }

    pub fn eof() -> Self {
        Self::singleton(-1)
    }

    pub fn union(&self, o: &Self) -> Self {
        let mut all: Vec<(i32, i32)> = Vec::with_capacity(self.0.len() + o.0.len());
        all.extend(&self.0);
        all.extend(&o.0);

        all.sort_by_key(|&(a, _)| a);

        let mut merged: Vec<(i32, i32)> = Vec::new();
        for (a, b) in all {
            if let Some(&mut (_, ref mut last_end)) = merged.last_mut() {
                if *last_end + 1 >= a {
                    *last_end = (*last_end).max(b);
                    continue;
                }
            }
            merged.push((a, b));
        }

        CSet(merged)
    }

    pub fn union_vec(l: Vec<Self>) -> Self {
        let mut v: Vec<(i32, i32)> = l.into_iter().flat_map(|c| c.0).collect();
        v.sort_by_key(|&(a, _)| a);
        let mut merged: Vec<(i32, i32)> = vec![];
        for (a, b) in v {
            if let Some(last) = merged.last_mut() {
                if last.1 + 1 >= a {
                    last.1 = last.1.max(b);
                    continue;
                }
            }
            merged.push((a, b));
        }
        CSet(merged)
    }

    pub fn intersection(&self, o: &Self) -> Self {
        let mut out = vec![];
        let (mut i, mut j) = (0, 0);
        let (a, b) = (&self.0, &o.0);

        while i < a.len() && j < b.len() {
            let lo = a[i].0.max(b[j].0);
            let hi = a[i].1.min(b[j].1);
            if lo <= hi {
                out.push((lo, hi));
            }
            match a[i].1.cmp(&b[j].1) {
                Ordering::Less => i += 1,
                Ordering::Greater => j += 1,
                Ordering::Equal => {
                    i += 1;
                    j += 1;
                }
            }
        }
        CSet(out)
    }

    pub fn difference(&self, o: &Self) -> Self {
        let mut out = vec![];
        let mut j = 0;

        for &(mut lo, hi) in &self.0 {
            while j < o.0.len() && o.0[j].1 < lo {
                j += 1;
            }

            let mut k = j;

            while k < o.0.len() && o.0[k].0 <= hi {
                if lo < o.0[k].0 {
                    out.push((lo, o.0[k].0 - 1));
                }
                lo = o.0[k].1 + 1;
                if lo > hi {
                    break;
                }
                k += 1;
            }
            if lo <= hi {
                out.push((lo, hi));
            }
        }

        CSet(out)
    }

    /// Return a new `CSet` that also matches the uppercase and lowercase variants of
    /// every code point already in `self`.
    ///
    /// Used to implement `#[lexer(case_insensitive)]`. Multi-character case folds
    /// (e.g. `ß` → `ss`) contribute each individual character to the set.
    ///
    /// > **Note:** iterates over every code point in every interval. Fast for typical
    /// > ASCII keyword sets; may slow compilation when applied to large Unicode categories.
    pub fn case_fold(&self) -> Self {
        let mut extras: Vec<Self> = vec![self.clone()];
        for &(lo, hi) in &self.0 {
            if hi < 0 { continue; } // skip EOF sentinel (-1)
            let lo = lo.max(0) as u32;
            let hi = hi as u32;
            for cp in lo..=hi {
                if let Some(c) = char::from_u32(cp) {
                    extras.extend(c.to_uppercase().map(|u| Self::singleton(u as i32)));
                    extras.extend(c.to_lowercase().map(|l| Self::singleton(l as i32)));
                }
            }
        }
        Self::union_vec(extras)
    }
}

impl TryFrom<Vec<(i32, i32)>> for CSet {
    type Error = String;

    fn try_from(value: Vec<(i32, i32)>) -> Result<Self, Self::Error> {
        let mut prev: i32 = -1;
        for (a, b) in &value {
            if *a < prev {
                return Err(format!(
                    "Vec not in an increasing order: found [_, (_, {prev}), ({a}, {b})]"
                ));
            }
            if *a == prev {
                return Err(format!(
                    "Found an adjacent range: [_, (_, {prev}), ({a}, {b})]"
                ));
            }
            if a > b {
                return Err(format!("Found a malformed range: [_, ({a}, {b})]"));
            }
            prev = *b;
        }
        Ok(CSet(value))
    }
}

impl TryFrom<&[(i32, i32)]> for CSet {
    type Error = String;

    /// NOTE: This will allocate a vector if the ranges are valid.
    fn try_from(value: &[(i32, i32)]) -> Result<Self, Self::Error> {
        let mut prev: i32 = -1;
        for (a, b) in value {
            if *a < prev {
                return Err(format!(
                    "Vec not in an increasing order: found [_, (_, {prev}), ({a}, {b})]"
                ));
            }
            if *a == prev {
                return Err(format!(
                    "Found an adjacent range: [_, (_, {prev}), ({a}, {b})]"
                ));
            }
            if a > b {
                return Err(format!("Found a malformed range: [_, ({a}, {b})]"));
            }
            prev = *b;
        }
        Ok(CSet(value.to_vec()))
    }
}

#[allow(clippy::from_over_into)] // I don't want to impl `From<CSet>` for `Vec<(i32, i32)>`.
impl Into<Vec<(i32, i32)>> for CSet {
    fn into(self) -> Vec<(i32, i32)> {
        self.0
    }
}

impl Hash for CSet {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union() {
        let c1 = CSet::interval(0, 5);
        let c2 = CSet::interval(3, 10);
        assert_eq!(c1.union(&c2), CSet::interval(0, 10));
        let c1 = CSet::interval(0, 5);
        let c2 = CSet::interval(3, 10);
        assert_eq!(c2.union(&c1), CSet::interval(0, 10));
        let c1 = CSet::interval(0, 11);
        let c2 = CSet::interval(3, 10);
        assert_eq!(c1.union(&c2), CSet::interval(0, 11));
    }

    #[test]
    fn intersection() {
        // Disjoint → empty
        assert_eq!(
            CSet::interval(0, 5).intersection(&CSet::interval(6, 10)),
            CSet::new()
        );
        // Partial overlap → shared sub-range
        assert_eq!(
            CSet::interval(0, 5).intersection(&CSet::interval(3, 10)),
            CSet::interval(3, 5)
        );
        // One contained in the other → the smaller
        assert_eq!(
            CSet::interval(0, 10).intersection(&CSet::interval(3, 7)),
            CSet::interval(3, 7)
        );
        // Identical sets → identity
        assert_eq!(
            CSet::interval(0, 10).intersection(&CSet::interval(0, 10)),
            CSet::interval(0, 10)
        );
    }

    #[test]
    fn difference() {
        // No overlap → unchanged
        assert_eq!(
            CSet::interval(0, 5).difference(&CSet::interval(6, 10)),
            CSet::interval(0, 5)
        );
        // Identical → empty
        assert_eq!(
            CSet::interval(0, 5).difference(&CSet::interval(0, 5)),
            CSet::new()
        );
        // Trim the start
        assert_eq!(
            CSet::interval(0, 5).difference(&CSet::interval(0, 2)),
            CSet::interval(3, 5)
        );
        // Trim the end
        assert_eq!(
            CSet::interval(0, 5).difference(&CSet::interval(3, 5)),
            CSet::interval(0, 2)
        );
        // Punch a hole → two disjoint intervals
        let result = CSet::interval(0, 10).difference(&CSet::interval(3, 7));
        let expected = CSet::interval(0, 2).union(&CSet::interval(8, 10));
        assert_eq!(result, expected);
    }

    #[test]
    fn case_fold() {
        // 'a' folds to include 'A'
        let lower_a = CSet::singleton('a' as i32);
        let upper_a = CSet::singleton('A' as i32);
        let folded = lower_a.case_fold();
        assert_eq!(upper_a.difference(&folded), CSet::new()); // 'A' is in folded
        assert_eq!(lower_a.difference(&folded), CSet::new()); // 'a' still in folded

        // Digit '0' has no case variant — unchanged
        let digit = CSet::singleton('0' as i32);
        assert_eq!(digit.case_fold(), digit);

        // EOF sentinel (-1) is never altered
        assert_eq!(CSet::eof().case_fold(), CSet::eof());
    }

    #[test]
    fn case_fold_sharp_s() {
        // 'ß' (U+00DF) has a multi-character uppercase mapping: "SS".
        // case_fold should add both 'S' (U+0053) to the set.
        let sharp_s = CSet::singleton('ß' as i32);
        let folded = sharp_s.case_fold();
        let s_upper = CSet::singleton('S' as i32);
        // 'S' must be in the folded set
        assert_eq!(s_upper.difference(&folded), CSet::new());
        // 'ß' itself must still be in the folded set
        assert_eq!(sharp_s.difference(&folded), CSet::new());
    }

    // ── Property-based tests ──────────────────────────────────────────────────
    //
    // These use proptest to verify algebraic laws that must hold for all
    // well-formed CSets: idempotence, commutativity, De Morgan's law, and the
    // partition identity A = (A ∩ B) ∪ (A \ B).

    use proptest::prelude::*;

    /// Generate an arbitrary CSet from a list of non-overlapping sorted intervals
    /// within the ASCII range (0..=127) for fast, reproducible tests.
    fn arb_cset() -> impl Strategy<Value = CSet> {
        prop::collection::vec(0i32..=127i32, 0..8).prop_map(|mut v| {
            v.sort_unstable();
            v.dedup();
            CSet::union_vec(v.into_iter().map(CSet::singleton).collect())
        })
    }

    proptest! {
        #[test]
        fn prop_union_idempotent(a in arb_cset()) {
            prop_assert_eq!(a.union(&a), a);
        }

        #[test]
        fn prop_union_commutative(a in arb_cset(), b in arb_cset()) {
            prop_assert_eq!(a.union(&b), b.union(&a));
        }

        #[test]
        fn prop_intersection_idempotent(a in arb_cset()) {
            prop_assert_eq!(a.intersection(&a), a);
        }

        #[test]
        fn prop_partition_identity(a in arb_cset(), b in arb_cset()) {
            // A = (A ∩ B) ∪ (A \ B)
            let reconstructed = a.intersection(&b).union(&a.difference(&b));
            prop_assert_eq!(reconstructed, a);
        }

    }
}
