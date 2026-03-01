// Copyright (c) 2025-2026 Emilien Lemaire <emilien.lem@icloud.com>
// SPDX-License-Identifier: LGPL-3.0-only
// Licensed under the GNU Lesser General Public License v3.0, with the
// ferrelex Generated Code Exception. See the LICENSE file at the root
// of this repository for the full license text and exception terms.

use ferrelex_core::cset::CSet;
use quote::quote;
use std::rc::Rc;

use crate::table_name;

/// Ranges narrower than `LIMIT` code points use a dense byte-indexed lookup table
/// (O(1) per character). Wider ranges fall back to a binary comparison tree (O(log n)).
const LIMIT: i32 = 8_192;

fn segments_of_partitions(partitions: &[CSet]) -> Vec<(i32, i32, i32)> {
    let mut segments = vec![];
    partitions
        .iter()
        .cloned()
        .enumerate()
        .for_each(|(i, cset)| {
            let part: Vec<_> = cset.into();
            part.into_iter()
                .for_each(|(a, b)| segments.push((a, b, i as i32)));
        });
    segments.sort_by(|(a1, _, _), (a2, _, _)| a1.cmp(a2));
    segments
}

#[derive(Debug, Clone)]
pub(crate) enum DecisionTree {
    Lte(i32, Rc<DecisionTree>, Rc<DecisionTree>),
    /// Dense lookup table.
    /// `offset` is the minimum character code subtracted before indexing.
    /// Each stored byte `v` encodes partition `base + v - 1`; `0` means no match.
    /// The runtime expression is `table[c - offset] as i32 + base - 1`.
    Table(i32, Vec<u8>, i32),
    Return(i32),
}

impl DecisionTree {
    /// Collapse `Lte` nodes whose both branches return the same value into a
    /// single `Return`. Applied after construction to eliminate redundant tests
    /// introduced when adjacent partitions share the same DFA state.
    pub fn simplify_decision_tree(self: Rc<Self>) -> Rc<Self> {
        use DecisionTree::*;
        match self.as_ref() {
            Table(_, _, _) | Return(_) => self.clone(),
            Lte(i, l, r) => match (l.as_ref(), r.as_ref()) {
                (Return(a), Return(b)) if a == b => l.clone(),
                _ => {
                    let l = l.clone().simplify_decision_tree();
                    let r = r.clone().simplify_decision_tree();
                    match (l.as_ref(), r.as_ref()) {
                        (Return(a), Return(b)) if a == b => l,
                        _ => Rc::new(Lte(*i, l.clone(), r.clone())),
                    }
                }
            },
        }
    }

    /// Build a pure binary comparison tree (`Lte` nodes) for a sorted list of
    /// `(lo, hi, partition_id)` segments.
    ///
    /// Used as a fallback when a dense lookup table would exceed [`LIMIT`] or
    /// when partition indices overflow a `u8`. Every leaf is a `Return(id)`;
    /// gaps between segments return `Return(-1)`.
    pub fn decision(l: &[(i32, i32, i32)]) -> Self {
        use DecisionTree::*;
        let l: Vec<_> = l
            .iter()
            .map(|(a, b, i)| (*a, *b, Rc::new(Return(*i))))
            .collect();

        // Pair adjacent segments bottom-up into `Lte` nodes.
        // Contiguous segments (b1 + 1 == a2) need no gap guard between them;
        // non-contiguous pairs insert a `Lte(a2 - 1, Return(-1), ...)` node
        // so that codes between the two segments return -1.
        fn merge2(l: Vec<(i32, i32, Rc<DecisionTree>)>) -> Vec<(i32, i32, Rc<DecisionTree>)> {
            let mut res = vec![];
            for elts in l.chunks(2) {
                let (a1, b1, d1) = &elts[0];
                if elts.len() == 1 {
                    res.push((*a1, *b1, d1.clone()));
                    continue;
                }
                let (a2, b2, d2) = &elts[1];
                let x = if b1 + 1 == *a2 {
                    d2.clone()
                } else {
                    Rc::new(Lte(a2 - 1, Rc::new(Return(-1)), d2.clone()))
                };
                res.push((*a1, *b2, Rc::new(Lte(*b1, d1.clone(), x.clone()))))
            }
            res
        }

        let mut l = merge2(l);

        loop {
            if l.len() == 1 {
                let (a, b, d) = l[0].clone();
                break Lte(
                    a - 1,
                    Rc::new(Return(-1)),
                    Rc::new(Lte(b, d.clone(), Rc::new(Return(-1)))),
                );
            }

            if l.is_empty() {
                break Return(-1);
            }

            l = merge2(l);
        }
    }

    /// Build the subtree for segments that overflowed the current table.
    ///
    /// Once `b >= LIMIT` appears in the sorted list all subsequent segments
    /// also exceed the limit, so we fall back to a binary decision tree.
    /// Otherwise every remaining segment can form the next table (O(1) lookup
    /// vs O(log n) comparisons), with partition indices offset by `base + 255`.
    fn rest_tree(rest: &[(i32, i32, i32)], base: i32) -> Self {
        use DecisionTree::*;
        match rest.first() {
            None => Return(-1),
            Some(&(_, b, _)) if b >= LIMIT => Self::decision(rest),
            _ => Self::_decision_table(rest.to_vec(), base + 255),
        }
    }

    fn _decision_table(l: Vec<(i32, i32, i32)>, base: i32) -> Self {
        use DecisionTree::*;
        let split = l
            .iter()
            .position(|&(_, b, i)| b >= LIMIT || i - base >= 255)
            .unwrap_or(l.len());

        let table = &l[..split];
        let rest = &l[split..];

        match table {
            [] => Self::decision(&l),
            &[(min, max, i)] => Lte(
                min - 1,
                Rc::new(Return(-1)),
                Rc::new(Lte(
                    max,
                    Rc::new(Return(i)),
                    Rc::new(Self::rest_tree(rest, base)),
                )),
            ),
            _ => {
                let min = table.iter().map(|&(a, _, _)| a).min().unwrap();
                let max = table.last().unwrap().1;

                let mut arr = vec![0u8; (max - min + 1) as usize];
                for &(a, b, i) in table {
                    for j in a..=b {
                        // Store (partition_id - base + 1): 0 is reserved for "no match".
                        // Decoded at runtime as `table[c] as i32 + base - 1` (see gen_tokens).
                        arr[(j - min) as usize] = ((i - base) as u8).strict_add(1);
                    }
                }

                Lte(
                    min - 1,
                    Rc::new(Return(-1)),
                    Rc::new(Lte(
                        max,
                        Rc::new(Table(min, arr, base)),
                        Rc::new(Self::rest_tree(rest, base)),
                    )),
                )
            }
        }
    }

    /// Prune unreachable branches using known value bounds.
    ///
    /// Called with `min = -1` (EOF sentinel) and `max = 0x10FFFF` (max code point)
    /// on the root. At each `Lte(i, yes, no)`, if `i >= max` then the `no` branch
    /// is unreachable (every value ≤ max satisfies `c <= i`), and vice-versa.
    /// This eliminates the sentinel guards inserted at the edges of the code point
    /// range that can never fire in practice.
    pub fn simplify(self: Rc<Self>, min: i32, max: i32) -> Rc<Self> {
        use DecisionTree::*;
        match self.as_ref() {
            Lte(i, yes, no) => {
                if *i >= max {
                    yes.clone().simplify(min, max)
                } else if *i < min {
                    no.clone().simplify(min, max)
                } else {
                    let simpl_yes = yes.clone().simplify(min, *i);
                    let simpl_no = no.clone().simplify(i + 1, max);
                    Rc::new(Lte(*i, simpl_yes.clone(), simpl_no.clone()))
                }
            }
            _ => self.clone(),
        }
    }

    pub fn decision_table(p: &[CSet]) -> Rc<Self> {
        let decision_table = Rc::new(Self::_decision_table(segments_of_partitions(p), 0));
        decision_table.simplify(-1, CSet::max_code())
    }

    pub fn gen_tokens(self: Rc<Self>) -> proc_macro2::TokenStream {
        use DecisionTree::*;
        match self.as_ref() {
            Lte(i, yes, no) => {
                let yes_toks = yes.clone().gen_tokens();
                let no_toks = no.clone().gen_tokens();
                quote! {
                    if c <= #i {
                        #yes_toks
                    } else {
                        #no_toks
                    }
                }
            }
            Return(i) => {
                quote! { #i }
            }
            Table(offset, t, base) => {
                let c = if *offset == 0 {
                    quote! {(c)}
                } else {
                    quote! {(c - #offset)}
                };
                let table_name = table_name(&t);
                if *base == 0 {
                    quote! { (#table_name[#c as usize] as i32 - 1) }
                } else {
                    quote! { (#table_name[#c as usize] as i32 + #base - 1) }
                }
            }
        }
    }
}
