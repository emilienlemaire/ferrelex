// Copyright (c) 2025-2026 Emilien Lemaire <emilien.lem@icloud.com>
// SPDX-License-Identifier: LGPL-3.0-only
// Licensed under the GNU Lesser General Public License v3.0, with the
// ferrelex Generated Code Exception. See the LICENSE file at the root
// of this repository for the full license text and exception terms.

use std::{cell::RefCell, rc::Rc};

use ferrelex_core::cset::CSet;
use rustc_hash::{FxHashMap, FxHashSet};
use syn::{
    BinOp, Expr, ExprBinary, ExprCall, ExprLit, ExprParen, ExprPath, ExprRange, ExprTuple, Lit,
    Pat, PatIdent, PatLit, PatOr, PatParen, PatRange, PatTuple, PatTupleStruct, Path, RangeLimits,
    parse_quote,
};

use crate::Env;

#[derive(Debug, Clone)]
pub(crate) enum Regex {
    Eps,
    Chars(CSet),
    Seq(Box<Regex>, Box<Regex>),
    Alt(Box<Regex>, Box<Regex>),
    Rep(Box<Regex>),
    Plus(Box<Regex>),
}

impl Regex {
    pub(crate) fn eps() -> Self {
        Self::Eps
    }

    pub(crate) fn seq(self, r: Self) -> Self {
        match (self, r) {
            (Self::Eps, r) => r,
            (s, Self::Eps) => s,
            (s, r) => Self::Seq(Box::new(s), Box::new(r)),
        }
    }

    pub fn chars(c: CSet) -> Self {
        Self::Chars(c)
    }

    pub(crate) fn rep(self) -> Self {
        Self::Rep(Box::new(self))
    }

    pub(crate) fn alt(self, r: Self) -> Self {
        match (&self, &r) {
            (Self::Chars(c1), Self::Chars(c2)) => Self::Chars(c1.union(c2)),
            _ => Self::Alt(Box::new(self), Box::new(r)),
        }
    }

    pub(crate) fn plus(self) -> Self {
        Self::Plus(Box::new(self))
    }

    pub(crate) fn compl(&self) -> Option<Self> {
        match self {
            Self::Chars(c) => Some(Self::Chars(CSet::any().difference(c))),
            _ => None,
        }
    }

    pub(crate) fn subtract(&self, r: &Self) -> Option<Self> {
        match (self, r) {
            (Self::Chars(c1), Self::Chars(c2)) => Some(Self::Chars(c1.difference(c2))),
            _ => None,
        }
    }

    pub(crate) fn intersect(&self, r: &Self) -> Option<Self> {
        match (self, r) {
            (Self::Chars(c1), Self::Chars(c2)) => Some(Self::Chars(c1.intersection(c2))),
            _ => None,
        }
    }

    pub(super) fn repeat(self, start: usize, end: usize) -> Regex {
        match (start, end) {
            (0, 0) => Regex::Eps,
            (0, n) => {
                let mut res = Regex::Eps;
                for _ in 1..=n {
                    let new = Regex::Eps.alt(self.clone().seq(res));
                    res = new;
                }
                res
            }
            (m, n) => {
                // Build r^m (mandatory prefix)
                let mut mandatory = self.clone();
                for _ in 1..m {
                    mandatory = self.clone().seq(mandatory);
                }
                // Append r{0, n-m} optional tail using the original atom, not
                // the accumulated mandatory part (which was the prior bug).
                if n == m {
                    mandatory
                } else {
                    mandatory.seq(self.repeat(0, n - m))
                }
            }
        }
    }
}

type NodeId = usize;

struct Node {
    _id: NodeId,
    eps: Vec<NodeId>,
    trans: Vec<(CSet, NodeId)>,
}

struct Nfa(Vec<Node>);

impl Node {
    fn new(id: NodeId) -> Self {
        Self {
            _id: id,
            eps: vec![],
            trans: vec![],
        }
    }
}

impl Nfa {
    pub(crate) fn new() -> Self {
        Self(vec![])
    }

    pub(crate) fn new_node(&mut self) -> NodeId {
        let id = self.0.len();
        self.0.push(Node::new(id));
        id
    }

    pub(crate) fn compile_re(&mut self, re: &Regex) -> (NodeId, NodeId) {
        let final_node = self.new_node();
        let start = self.build(re, final_node);
        (start, final_node)
    }

    /// Recursively build an NFA sub-graph for `re` and return its start node.
    ///
    /// `succ` is the NFA node that every accepting path through the sub-graph must
    /// eventually reach — the *continuation* in a continuation-passing idiom inherited
    /// from the original OCaml source. Sequences are built right-to-left: `Seq(r1, r2)`
    /// first builds `r2` (with the original `succ`), then builds `r1` with the start of
    /// `r2` as its new successor.
    fn build(&mut self, re: &Regex, succ: NodeId) -> NodeId {
        match re {
            Regex::Eps => succ,
            Regex::Chars(c) => {
                let n = self.new_node();
                self.0[n].trans.push((c.clone(), succ));
                n
            }
            Regex::Seq(r1, r2) => {
                let mid = self.build(r2, succ);
                self.build(r1, mid)
            }
            Regex::Alt(r1, r2) => {
                let nr1 = self.build(r1, succ);
                let nr2 = self.build(r2, succ);

                match (self.is_chars(succ, nr1), self.is_chars(succ, nr2)) {
                    (Some(c1), Some(c2)) => {
                        let n = self.new_node();
                        self.0[n].trans.push((c1.union(&c2), succ));
                        n
                    }
                    _ => {
                        let n = self.new_node();
                        self.0[n].eps.extend([nr1, nr2]);
                        n
                    }
                }
            }
            Regex::Rep(r) => {
                let n = self.new_node();
                let nr = self.build(r, n);
                self.0[n].eps.extend([nr, succ]);
                n
            }
            Regex::Plus(r) => {
                let n = self.new_node();
                let nr = self.build(r, n);
                self.0[n].eps.extend([nr, succ]);
                nr
            }
        }
    }

    fn is_chars(&self, final_node: NodeId, node: NodeId) -> Option<CSet> {
        let n = &self.0[node];
        match (n.eps.as_slice(), n.trans.as_slice()) {
            ([], [(c, f)]) if *f == final_node => Some(c.clone()),
            _ => None,
        }
    }

    fn add_node(&self, visited: &mut FxHashSet<NodeId>, id: NodeId) {
        if !visited.insert(id) {
            return;
        }

        for &eps_id in &self.0[id].eps {
            self.add_node(visited, eps_id);
        }
    }

    fn add_nodes(&self, ids: &[NodeId]) -> Vec<NodeId> {
        let mut visited: FxHashSet<NodeId> = Default::default();
        for &id in ids {
            self.add_node(&mut visited, id);
        }

        let mut state: Vec<NodeId> = visited.into_iter().collect();
        state.sort_unstable();
        state
    }

    /// Compute all outgoing transitions for an NFA *state set* (a DFA state).
    ///
    /// Collects every character-transition reachable from any node in `state`,
    /// then partitions the combined character space so that each returned
    /// `(CSet, target_state)` pair covers a maximal set of code points that all
    /// lead to exactly the same epsilon-closed target node set. This is the
    /// core of the standard subset construction step.
    fn transition(&self, state: &[NodeId]) -> Vec<(CSet, Vec<NodeId>)> {
        let mut trans: Vec<_> = state
            .iter()
            .flat_map(|&id| self.0[id].trans.iter().cloned())
            .collect();

        trans.sort_by_key(|&(_, id)| id);
        let mut merged: Vec<(CSet, NodeId)> = vec![];
        for (c, n) in trans {
            if let Some((prev_c, prev_n)) = merged.last_mut() {
                if *prev_n == n {
                    *prev_c = prev_c.union(&c);
                    continue;
                }
            }
            merged.push((c, n));
        }

        let mut all = CSet::new();
        let mut parts: Vec<(CSet, Vec<NodeId>)> = vec![];

        for (c, n) in &merged {
            let mut new_parts: Vec<(CSet, Vec<NodeId>)> = vec![];
            new_parts.push((c.difference(&all), vec![*n]));

            for (c1, ns) in &parts {
                let inter = c1.intersection(c);
                if !inter.is_empty() {
                    let mut ns1 = ns.clone();
                    ns1.push(*n);
                    new_parts.push((inter, ns1))
                }
                let diff = c1.difference(c);
                if !diff.is_empty() {
                    new_parts.push((diff, ns.clone()))
                }
            }

            all = all.union(c);
            parts = new_parts
                .into_iter()
                .filter(|(c, _)| !c.is_empty())
                .collect();
        }

        let mut result: Vec<(CSet, Vec<NodeId>)> = parts
            .into_iter()
            .map(|(c, ns)| (c, self.add_nodes(&ns)))
            .collect();

        result.sort_by(|(c1, _), (c2, _)| c1.cmp(c2));
        result
    }
}

/// Compile a slice of regexes into a DFA via NFA subset construction.
///
/// Each entry in `regexs` corresponds to one match arm (in source order). The function:
/// 1. Converts every regex to an NFA sub-graph sharing one arena.
/// 2. Runs the standard subset construction (BFS worklist) to enumerate DFA states.
/// 3. Returns one entry per DFA state as `(transitions, accepting_flags)`:
///    - `transitions`: `(CSet, next_state)` pairs — the character classes that lead
///      out of this state and the index of the target state.
///    - `accepting_flags[i]`: `true` when this DFA state contains the accepting node
///      of regex `i`, meaning the DFA accepts pattern `i` here.
///
/// State 0 is always the initial state.
pub(super) fn compile(regexs: &[Regex]) -> Vec<(Vec<(CSet, i32)>, Vec<bool>)> {
    let mut nfa = Nfa::new();

    let compiled: Vec<(NodeId, NodeId)> = regexs.iter().map(|re| nfa.compile_re(re)).collect();

    let mut state_map: FxHashMap<Vec<NodeId>, i32> = Default::default();
    let mut worklist: Vec<Vec<NodeId>> = vec![];
    let mut states_def: Vec<(Vec<(CSet, i32)>, Vec<bool>)> = vec![];

    let intern = |state: Vec<NodeId>,
                  worklist: &mut Vec<Vec<NodeId>>,
                  states_def: &mut Vec<(Vec<(CSet, i32)>, Vec<bool>)>,
                  state_map: &mut FxHashMap<Vec<NodeId>, i32>|
     -> i32 {
        let next = state_map.len() as i32;
        *state_map.entry(state.clone()).or_insert_with(|| {
            worklist.push(state);
            states_def.push((vec![], vec![]));
            next
        })
    };

    let init = nfa.add_nodes(&compiled.iter().map(|(s, _)| *s).collect::<Vec<_>>());

    let i = intern(init, &mut worklist, &mut states_def, &mut state_map);
    assert_eq!(i, 0);

    let mut cursor = 0;
    while cursor < worklist.len() {
        let trans = nfa.transition(&worklist[cursor]);
        let trans: Vec<_> = trans
            .into_iter()
            .map(|(c, target)| {
                let j = intern(target, &mut worklist, &mut states_def, &mut state_map);
                (c, j)
            })
            .collect();
        let finals: Vec<_> = compiled
            .iter()
            .map(|(_, f)| worklist[cursor].binary_search(f).is_ok())
            .collect();
        states_def[cursor] = (trans, finals);
        cursor += 1;
    }

    states_def
}

/// Recursively expand every `Chars` leaf in `r` to include case variants.
/// Used when `#[lexer(case_insensitive)]` is set.
pub(super) fn case_fold_regex(r: Regex) -> Regex {
    match r {
        Regex::Chars(c) => Regex::Chars(c.case_fold()),
        Regex::Seq(a, b) => {
            Regex::Seq(Box::new(case_fold_regex(*a)), Box::new(case_fold_regex(*b)))
        }
        Regex::Alt(a, b) => {
            Regex::Alt(Box::new(case_fold_regex(*a)), Box::new(case_fold_regex(*b)))
        }
        Regex::Rep(r) => Regex::Rep(Box::new(case_fold_regex(*r))),
        Regex::Plus(r) => Regex::Plus(Box::new(case_fold_regex(*r))),
        Regex::Eps => Regex::Eps,
    }
}

pub(super) fn builtin_regex() -> Env {
    let mut regex: FxHashMap<String, Regex> = FxHashMap::default();
    regex.insert("any".into(), Regex::chars(CSet::any()));
    regex.insert("eof".into(), Regex::chars(CSet::eof()));

    // ASCII-only character class shorthands (suffix _ascii makes the scope explicit;
    // Unicode equivalents are accessible via Unicode category/property identifiers).
    let digit_ascii = CSet::interval('0' as i32, '9' as i32);
    let upper_ascii = CSet::interval('A' as i32, 'Z' as i32);
    let lower_ascii = CSet::interval('a' as i32, 'z' as i32);
    let alpha_ascii = upper_ascii.union(&lower_ascii);
    let alnum_ascii = alpha_ascii.union(&digit_ascii);
    let whitespace_ascii = CSet::singleton(' ' as i32)
        .union(&CSet::singleton('\t' as i32))
        .union(&CSet::singleton('\n' as i32))
        .union(&CSet::singleton('\r' as i32));
    let word_ascii = alnum_ascii.union(&CSet::singleton('_' as i32));

    regex.insert("digit_ascii".into(), Regex::chars(digit_ascii));
    regex.insert("upper_ascii".into(), Regex::chars(upper_ascii));
    regex.insert("lower_ascii".into(), Regex::chars(lower_ascii));
    regex.insert("alpha_ascii".into(), Regex::chars(alpha_ascii));
    regex.insert("alnum_ascii".into(), Regex::chars(alnum_ascii));
    regex.insert("whitespace_ascii".into(), Regex::chars(whitespace_ascii));
    regex.insert("word_ascii".into(), Regex::chars(word_ascii));

    regex
}

pub(super) fn regex_of_expr(env: Rc<RefCell<Env>>, expr: Expr) -> Result<Regex, syn::Error> {
    match expr.clone() {
        Expr::Paren(ExprParen { expr, .. }) => regex_of_expr(env.clone(), *expr),
        Expr::Binary(ExprBinary {
            left: lhs,
            op: BinOp::BitOr(_),
            right: rhs,
            ..
        }) => {
            let r1 = regex_of_expr(env.clone(), *lhs.clone())?;
            let r2 = regex_of_expr(env.clone(), *rhs.clone())?;
            Ok(r1.alt(r2))
        }
        Expr::Tuple(ExprTuple { elems, .. }) => match elems.clone().first() {
            Some(elem) => elems.into_iter().skip(1).fold(
                regex_of_expr(env.clone(), elem.clone()),
                |acc, elem| {
                    let regex = regex_of_expr(env.clone(), elem)?;
                    let acc = acc?;
                    Ok(acc.seq(regex))
                },
            ),
            None => Err(syn::Error::new_spanned(expr, "Invalid empty tuple")),
        },
        Expr::Call(ExprCall { func, args, .. }) => {
            if args.is_empty() {
                return Err(syn::Error::new_spanned(
                    args.clone(),
                    "Expecting at least one argument on a function like regex expression",
                ));
            }
            match *func {
                e if e == parse_quote! {Star} => {
                    if args.len() > 1 {
                        return Err(syn::Error::new_spanned(
                            args,
                            "`Star` is only expecting one argument, if you need to concatenate multiple \
                        regex, please use a tuple as argument.",
                        ));
                    }
                    let arg_regex = regex_of_expr(env.clone(), args[0].clone())?;
                    Ok(arg_regex.rep())
                }
                e if e == parse_quote! {Plus} => {
                    if args.len() > 1 {
                        return Err(syn::Error::new_spanned(
                            args,
                            "`Plus` is only expecting one argument, if you need to concatenate multiple \
                        regex, please use a tuple as argument.",
                        ));
                    }
                    let args_regex = regex_of_expr(env.clone(), args[0].clone())?;
                    Ok(args_regex.plus())
                }
                e if e == parse_quote! {Rep} => {
                    if args.len() != 2 {
                        return Err(syn::Error::new_spanned(
                            args,
                            "`Rep` expects two arguments, a regex and a repeat count.",
                        ));
                    }
                    match &args[1] {
                        Expr::Range(ExprRange {
                            start: Some(start),
                            end: Some(end),
                            limits,
                            ..
                        }) => match (*start.clone(), *end.clone()) {
                            (
                                Expr::Lit(ExprLit {
                                    lit: Lit::Int(start_lit),
                                    ..
                                }),
                                Expr::Lit(ExprLit {
                                    lit: Lit::Int(end_lit),
                                    ..
                                }),
                            ) => {
                                match (
                                    start_lit.base10_parse::<usize>(),
                                    end_lit.base10_parse::<usize>(),
                                ) {
                                    (Ok(start), Ok(end)) => {
                                        if start > end {
                                            return Err(syn::Error::new_spanned(
                                                args[1].clone(),
                                                format!(
                                                    "Range expression for `Rep` must be incrementing, \
                                                got {start} > {end}"
                                                ),
                                            ));
                                        }

                                        let regex = regex_of_expr(env.clone(), args[0].clone())?;
                                        let rep = if let RangeLimits::HalfOpen(_) = limits {
                                            if start > end - 1 {
                                                return Err(syn::Error::new_spanned(
                                                    args[1].clone(),
                                                    format!(
                                                        "Range expression for `Rep` must be incrementing, \
                                                got {start} > {end}"
                                                    ),
                                                ));
                                            }
                                            regex.repeat(start, end - 1)
                                        } else {
                                            regex.repeat(start, end)
                                        };
                                        Ok(rep)
                                    }
                                    (Err(e), _) | (_, Err(e)) => {
                                        Err(syn::Error::new_spanned(
                                            args[1].clone(),
                                            format!(
                                                "Could not parse integer literal as `usize` value: {e}",
                                            ),
                                        ))
                                    }
                                }
                            }
                            _ => Err(syn::Error::new_spanned(
                                args[1].clone(),
                                "Expecting bound of `Rep` range to be integer literals.",
                            )),
                        },
                        Expr::Range(ExprRange {
                            start: None,
                            end: Some(_),
                            ..
                        })
                        | Expr::Range(ExprRange {
                            start: Some(_),
                            end: None,
                            ..
                        }) => Err(syn::Error::new_spanned(
                            args[1].clone(),
                            "Expecting a full range as second argument of `Rep` operator.",
                        )),
                        Expr::Lit(ExprLit {
                            lit: Lit::Int(lit_int),
                            ..
                        }) => match lit_int.base10_parse::<usize>() {
                            Err(e) => {
                                Err(syn::Error::new_spanned(
                                    args[1].clone(),
                                    format!(
                                        "Could not parse integer literal as `usize` value: {e}",
                                    ),
                                ))
                            }
                            Ok(v) => {
                                let regex = regex_of_expr(env.clone(), args[0].clone())?;
                                Ok(regex.repeat(v, v))
                            }
                        },
                        _ => Err(syn::Error::new_spanned(
                            args[1].clone(),
                            "Expecting a range or a integer literal as second argument of the `Rep` \
                        operator.",
                        )),
                    }
                }
                e if e == parse_quote! {Opt} => {
                    if args.len() > 1 {
                        return Err(syn::Error::new_spanned(
                            args.clone(),
                            "Expecting a single argument for the `Opt` operator",
                        ));
                    }
                    let regex = regex_of_expr(env.clone(), args[0].clone())?;
                    Ok(Regex::eps().alt(regex))
                }
                e if e == parse_quote! {Compl} => {
                    if args.len() > 1 {
                        return Err(syn::Error::new_spanned(
                            args,
                            "`Compl` is only expecting one argument, if you need to concatenate multiple \
                        regex, please use a tuple as argument.",
                        ));
                    }
                    let regex = regex_of_expr(env.clone(), args[0].clone())?;
                    match regex.compl() {
                        Some(c) => Ok(c),
                        None => Err(syn::Error::new_spanned(
                            args.clone(),
                            "`Compl` expects only single character length regex.",
                        )),
                    }
                }
                e if e == parse_quote! {Sub} => {
                    if args.len() != 2 {
                        return Err(syn::Error::new_spanned(
                            args.clone(),
                            "`Sub` operator is expecting two arguments",
                        ));
                    }
                    let r1 = regex_of_expr(env.clone(), args[0].clone())?;
                    let r2 = regex_of_expr(env.clone(), args[1].clone())?;
                    match r1.subtract(&r2) {
                        Some(r) => Ok(r),
                        None => Err(syn::Error::new_spanned(
                            args,
                            "`Sub` operator can only be applied to single character length regexes",
                        )),
                    }
                }
                e if e == parse_quote! {Intersect} => {
                    if args.len() != 2 {
                        return Err(syn::Error::new_spanned(
                            args.clone(),
                            "`Intersect` operator is expecting two arguments",
                        ));
                    }
                    let r1 = regex_of_expr(env.clone(), args[0].clone())?;
                    let r2 = regex_of_expr(env.clone(), args[1].clone())?;
                    match r1.intersect(&r2) {
                        Some(r) => Ok(r),
                        None => Err(syn::Error::new_spanned(
                            args,
                            "`Intersect` operator can only be applied to single character length regexes",
                        )),
                    }
                }
                e if e == parse_quote! {AnyOf} || e == parse_quote! {Chars} => {
                    if args.len() > 1 {
                        return Err(syn::Error::new_spanned(
                            args,
                            "`AnyOf` operator only accepts one argument.",
                        ));
                    }
                    match &args[0] {
                        Expr::Lit(ExprLit {
                            lit: Lit::Str(lit_str),
                            ..
                        }) => {
                            let string = lit_str.value();
                            let c = string
                                .chars()
                                .map(|c| CSet::singleton(c as i32))
                                .fold(CSet::new(), |acc, s| acc.union(&s));
                            Ok(Regex::chars(c))
                        }
                        _ => Err(syn::Error::new_spanned(
                            args,
                            "`AnyOf` operator only accepts a str literal as argument.",
                        )),
                    }
                }
                func => Err(syn::Error::new_spanned(
                    func,
                    "Invalid expression for a regex",
                )),
            }
        } // Expr::Call
        Expr::Range(ExprRange {
            start, limits, end, ..
        }) => match (start, end) {
            (Some(start), Some(end)) => match (*start, *end) {
                (
                    Expr::Lit(ExprLit {
                        lit: Lit::Char(c1), ..
                    }),
                    Expr::Lit(ExprLit {
                        lit: Lit::Char(c2), ..
                    }),
                ) => {
                    let (c1, c2) = (c1.value(), c2.value());
                    let i1 = c1 as i32;
                    let i2_raw = c2 as i32;
                    let i2 = if let RangeLimits::HalfOpen(_) = limits {
                        i2_raw - 1
                    } else {
                        i2_raw
                    };
                    if i2 < i1 {
                        return Err(syn::Error::new_spanned(
                            expr.clone(),
                            "Empty character range.",
                        ));
                    }
                    Ok(Regex::chars(CSet::interval(i1, i2)))
                }
                (
                    Expr::Lit(ExprLit {
                        lit: Lit::Int(i1_lit),
                        ..
                    }),
                    Expr::Lit(ExprLit {
                        lit: Lit::Int(i2_lit),
                        ..
                    }),
                ) => match (i1_lit.base10_parse::<i32>(), i2_lit.base10_parse::<i32>()) {
                    (Ok(i1), Ok(i2_raw)) => {
                        if i1 < 0 || i1 > CSet::max_code() {
                            return Err(syn::Error::new_spanned(
                                i1_lit.clone(),
                                "Invalid Unicode character code: {i1:0x4}",
                            ));
                        }
                        if i2_raw < 0 || i2_raw > CSet::max_code() {
                            return Err(syn::Error::new_spanned(
                                i2_lit.clone(),
                                "Invalid Unicode character code: {i2_raw:0x4}",
                            ));
                        }
                        let i2 = if let RangeLimits::HalfOpen(_) = limits {
                            i2_raw - 1
                        } else {
                            i2_raw
                        };
                        if i2 < i1 {
                            return Err(syn::Error::new_spanned(expr, "Empty character range."));
                        }
                        Ok(Regex::chars(CSet::interval(i1, i2)))
                    }
                    (Err(e), _) | (_, Err(e)) => Err(syn::Error::new_spanned(
                        expr,
                        format!(
                            "An integer range should have integer parsable as `i32`: {e}",
                        ),
                    )),
                },
                _ => Err(syn::Error::new_spanned(
                    expr.clone(),
                    "A range expr must be either bounded with char literal or integer literal.",
                )),
            },
            (None, _) | (_, None) => Err(syn::Error::new_spanned(
                expr.clone(),
                "A range expr expects bound on both sides.",
            )),
        },
        Expr::Lit(ExprLit { lit, .. }) => match lit {
            Lit::Str(lit_str) => Ok(lit_str
                .value()
                .chars()
                .map(|i| CSet::singleton(i as i32))
                .fold(Regex::Eps, |acc, s| {
                    let c = Regex::chars(s);
                    acc.seq(c)
                })),
            Lit::Char(lit_char) => {
                let c = lit_char.value();
                Ok(Regex::chars(CSet::singleton(c as i32)))
            }
            Lit::Int(lit_int) => match lit_int.base10_parse::<i32>() {
                Ok(c) => {
                    if c < 0 || c > CSet::max_code() {
                        return Err(syn::Error::new_spanned(
                            lit_int,
                            "Invalid Unicode code point: {c:0x4}",
                        ));
                    }
                    Ok(Regex::chars(CSet::singleton(c)))
                }
                Err(e) => Err(syn::Error::new_spanned(
                    lit_int,
                    format!(
                        "Expecting int in regex to be parsable as `i32` ({e}).",
                    ),
                )),
            },
            _ => Err(syn::Error::new_spanned(
                expr.clone(),
                "Invalid expression for a regex.",
            )),
        },
        Expr::Path(ExprPath {
            qself: None,
            path:
                Path {
                    leading_colon: None,
                    segments,
                },
            ..
        }) => {
            if segments.len() > 1 {
                return Err(syn::Error::new_spanned(
                    segments,
                    "Path expression must have only one segment in regex.",
                ));
            }
            let name = segments[0].ident.to_string();
            if let Some(c) = env.borrow().get(&name) {
                Ok(c.clone())
            } else if let Some(cset) = ferrelex_core::unicode_props::from_name(&name)
                .or_else(|| ferrelex_core::unicode_categories::from_name(&name))
            {
                Ok(Regex::chars(cset))
            } else {
                Err(syn::Error::new_spanned(
                    segments,
                    format!("Unbound regex: {name}."),
                ))
            }
        }
        _ => Err(syn::Error::new_spanned(
            expr,
            "Invalid expression for a regex",
        )),
    }
}

/// Pattern-context counterpart of [`regex_of_expr`].
///
/// Handles the same set of operators (`Star`, `Plus`, `Rep`, `Opt`, `Compl`,
/// `Sub`, `Intersect`, `AnyOf`, ranges, literals, and identifiers) but operates
/// on [`syn::Pat`] nodes instead of [`syn::Expr`] nodes, so that regex constants
/// can be referenced inside `match` arm patterns.
pub(super) fn regex_of_pattern(env: Rc<RefCell<Env>>, pat: Pat) -> Result<Regex, syn::Error> {
    match pat.clone() {
        Pat::Paren(PatParen { pat, .. }) => regex_of_pattern(env.clone(), *pat),
        Pat::Or(PatOr { cases, .. }) => {
            let mut regex = regex_of_pattern(env.clone(), cases[0].clone())?;
            for pat in cases.iter().skip(1) {
                let new_regex = regex_of_pattern(env.clone(), pat.clone())?;
                regex = regex.alt(new_regex);
            }
            Ok(regex)
        }
        Pat::Tuple(PatTuple { elems, .. }) => match elems.clone().first() {
            Some(elem) => elems.into_iter().skip(1).fold(
                regex_of_pattern(env.clone(), elem.clone()),
                |acc, elem| {
                    let regex = regex_of_pattern(env.clone(), elem)?;
                    let acc = acc?;
                    Ok(acc.seq(regex))
                },
            ),
            None => Err(syn::Error::new_spanned(pat, "Invalid empty tuple")),
        },
        Pat::TupleStruct(PatTupleStruct { path, elems, .. }) => {
            if elems.is_empty() {
                return Err(syn::Error::new_spanned(
                    elems.clone(),
                    "Expecting at least one argument on a function like regex expression",
                ));
            }
            match path {
                e if e == parse_quote! {Star} => {
                    if elems.len() > 1 {
                        return Err(syn::Error::new_spanned(
                            elems,
                            "`Star` is only expecting one argument, if you need to concatenate multiple \
                        regex, please use a tuple as argument.",
                        ));
                    }
                    let arg_regex = regex_of_pattern(env.clone(), elems[0].clone())?;
                    Ok(arg_regex.rep())
                }
                e if e == parse_quote! {Plus} => {
                    if elems.len() > 1 {
                        return Err(syn::Error::new_spanned(
                            elems,
                            "`Plus` is only expecting one argument, if you need to concatenate multiple \
                        regex, please use a tuple as argument.",
                        ));
                    }
                    let elems_regex = regex_of_pattern(env.clone(), elems[0].clone())?;
                    Ok(elems_regex.plus())
                }
                e if e == parse_quote! {Rep} => {
                    if elems.len() != 2 {
                        return Err(syn::Error::new_spanned(
                            elems,
                            "`Rep` expects two arguments, a regex and a repeat count.",
                        ));
                    }
                    match &elems[1] {
                        Pat::Range(PatRange {
                            start: Some(start),
                            end: Some(end),
                            limits,
                            ..
                        }) => match (*start.clone(), *end.clone()) {
                            (
                                Expr::Lit(ExprLit {
                                    lit: Lit::Int(start_lit),
                                    ..
                                }),
                                Expr::Lit(ExprLit {
                                    lit: Lit::Int(end_lit),
                                    ..
                                }),
                            ) => {
                                match (
                                    start_lit.base10_parse::<usize>(),
                                    end_lit.base10_parse::<usize>(),
                                ) {
                                    (Ok(start), Ok(end)) => {
                                        if start > end {
                                            return Err(syn::Error::new_spanned(
                                                elems[1].clone(),
                                                format!(
                                                    "Range expression for `Rep` must be incrementing, \
                                                got {start} > {end}"
                                                ),
                                            ));
                                        }

                                        let regex =
                                            regex_of_pattern(env.clone(), elems[0].clone())?;
                                        let rep = if let RangeLimits::HalfOpen(_) = limits {
                                            if start > end - 1 {
                                                return Err(syn::Error::new_spanned(
                                                    elems[1].clone(),
                                                    format!(
                                                        "Range expression for `Rep` must be incrementing, \
                                                got {start} > {end}"
                                                    ),
                                                ));
                                            }
                                            regex.repeat(start, end - 1)
                                        } else {
                                            regex.repeat(start, end)
                                        };
                                        Ok(rep)
                                    }
                                    (Err(e), _) | (_, Err(e)) => {
                                        Err(syn::Error::new_spanned(
                                            elems[1].clone(),
                                            format!(
                                                "Could not parse integer literal as `usize` value: {e}",
                                            ),
                                        ))
                                    }
                                }
                            }
                            _ => Err(syn::Error::new_spanned(
                                elems[1].clone(),
                                "Expecting bound of `Rep` range to be integer literals.",
                            )),
                        },
                        Pat::Range(PatRange {
                            start: None,
                            end: Some(_),
                            ..
                        })
                        | Pat::Range(PatRange {
                            start: Some(_),
                            end: None,
                            ..
                        }) => Err(syn::Error::new_spanned(
                            elems[1].clone(),
                            "Expecting a full range as second argument of `Rep` operator.",
                        )),
                        Pat::Lit(PatLit {
                            lit: Lit::Int(lit_int),
                            ..
                        }) => match lit_int.base10_parse::<usize>() {
                            Err(e) => {
                                Err(syn::Error::new_spanned(
                                    elems[1].clone(),
                                    format!(
                                        "Could not parse integer literal as `usize` value: {e}",
                                    ),
                                ))
                            }
                            Ok(v) => {
                                let regex = regex_of_pattern(env.clone(), elems[0].clone())?;
                                Ok(regex.repeat(v, v))
                            }
                        },
                        _ => Err(syn::Error::new_spanned(
                            elems[1].clone(),
                            "Expecting a range or a integer literal as second argument of the `Rep` \
                        operator.",
                        )),
                    }
                }
                e if e == parse_quote! {Opt} => {
                    if elems.len() > 1 {
                        return Err(syn::Error::new_spanned(
                            elems.clone(),
                            "Expecting a single argument for the `Opt` operator",
                        ));
                    }
                    let regex = regex_of_pattern(env.clone(), elems[0].clone())?;
                    Ok(Regex::eps().alt(regex))
                }
                e if e == parse_quote! {Compl} => {
                    if elems.len() > 1 {
                        return Err(syn::Error::new_spanned(
                            elems,
                            "`Compl` is only expecting one argument, if you need to concatenate multiple \
                        regex, please use a tuple as argument.",
                        ));
                    }
                    let regex = regex_of_pattern(env.clone(), elems[0].clone())?;
                    match regex.compl() {
                        Some(c) => Ok(c),
                        None => Err(syn::Error::new_spanned(
                            elems.clone(),
                            "`Compl` expects only single character length regex.",
                        )),
                    }
                }
                e if e == parse_quote! {Sub} => {
                    if elems.len() != 2 {
                        return Err(syn::Error::new_spanned(
                            elems.clone(),
                            "`Sub` operator is expecting two arguments",
                        ));
                    }
                    let r1 = regex_of_pattern(env.clone(), elems[0].clone())?;
                    let r2 = regex_of_pattern(env.clone(), elems[1].clone())?;
                    match r1.subtract(&r2) {
                        Some(r) => Ok(r),
                        None => Err(syn::Error::new_spanned(
                            elems,
                            "`Sub` operator can only be applied to single character length regexes",
                        )),
                    }
                }
                e if e == parse_quote! {Intersect} => {
                    if elems.len() != 2 {
                        return Err(syn::Error::new_spanned(
                            elems.clone(),
                            "`Intersect` operator is expecting two arguments",
                        ));
                    }
                    let r1 = regex_of_pattern(env.clone(), elems[0].clone())?;
                    let r2 = regex_of_pattern(env.clone(), elems[1].clone())?;
                    match r1.intersect(&r2) {
                        Some(r) => Ok(r),
                        None => Err(syn::Error::new_spanned(
                            elems,
                            "`Intersect` operator can only be applied to single character length regexes",
                        )),
                    }
                }
                e if e == parse_quote! {AnyOf} || e == parse_quote! {Chars} => {
                    if elems.len() > 1 {
                        return Err(syn::Error::new_spanned(
                            elems,
                            "`AnyOf` operator only accepts one argument.",
                        ));
                    }
                    match &elems[0] {
                        Pat::Lit(PatLit {
                            lit: Lit::Str(lit_str),
                            ..
                        }) => {
                            let string = lit_str.value();
                            let c = string
                                .chars()
                                .map(|c| CSet::singleton(c as i32))
                                .fold(CSet::new(), |acc, s| acc.union(&s));
                            Ok(Regex::chars(c))
                        }
                        _ => Err(syn::Error::new_spanned(
                            elems,
                            "`AnyOf` operator only accepts a str literal as argument.",
                        )),
                    }
                }
                func => Err(syn::Error::new_spanned(
                    func,
                    "Invalid expression for a regex",
                )),
            }
        } // Expr::Call
        Pat::Range(PatRange {
            start, limits, end, ..
        }) => match (start, end) {
            (Some(start), Some(end)) => match (*start, *end) {
                (
                    Expr::Lit(PatLit {
                        lit: Lit::Char(c1), ..
                    }),
                    Expr::Lit(PatLit {
                        lit: Lit::Char(c2), ..
                    }),
                ) => {
                    let (c1, c2) = (c1.value(), c2.value());
                    let i1 = c1 as i32;
                    let i2_raw = c2 as i32;
                    let i2 = if let RangeLimits::HalfOpen(_) = limits {
                        i2_raw - 1
                    } else {
                        i2_raw
                    };
                    if i2 < i1 {
                        return Err(syn::Error::new_spanned(
                            pat.clone(),
                            "Empty character range.",
                        ));
                    }
                    Ok(Regex::chars(CSet::interval(i1, i2)))
                }
                (
                    Expr::Lit(PatLit {
                        lit: Lit::Int(i1_lit),
                        ..
                    }),
                    Expr::Lit(PatLit {
                        lit: Lit::Int(i2_lit),
                        ..
                    }),
                ) => match (i1_lit.base10_parse::<i32>(), i2_lit.base10_parse::<i32>()) {
                    (Ok(i1), Ok(i2_raw)) => {
                        if i1 < 0 || i1 > CSet::max_code() {
                            return Err(syn::Error::new_spanned(
                                i1_lit.clone(),
                                "Invalid Unicode character code: {i1:0x4}",
                            ));
                        }
                        if i2_raw < 0 || i2_raw > CSet::max_code() {
                            return Err(syn::Error::new_spanned(
                                i2_lit.clone(),
                                "Invalid Unicode character code: {i2_raw:0x4}",
                            ));
                        }
                        let i2 = if let RangeLimits::HalfOpen(_) = limits {
                            i2_raw - 1
                        } else {
                            i2_raw
                        };
                        if i2 < i1 {
                            return Err(syn::Error::new_spanned(
                                pat.clone(),
                                "Empty character range.",
                            ));
                        }
                        Ok(Regex::chars(CSet::interval(i1, i2)))
                    }
                    (Err(e), _) | (_, Err(e)) => Err(syn::Error::new_spanned(
                        pat,
                        format!(
                            "An integer range should have integer parsable as `i32`: {e}",
                        ),
                    )),
                },
                _ => Err(syn::Error::new_spanned(
                    pat.clone(),
                    "A range expr must be either bounded with char literal or integer literal.",
                )),
            },
            (None, _) | (_, None) => Err(syn::Error::new_spanned(
                pat.clone(),
                "A range expr expects bound on both sides.",
            )),
        },
        Pat::Lit(PatLit { lit, .. }) => match lit {
            Lit::Str(lit_str) => Ok(lit_str
                .value()
                .chars()
                .map(|c| CSet::singleton(c as i32))
                .fold(Regex::eps(), |acc, s| {
                    let c = Regex::chars(s);
                    acc.seq(c)
                })),
            Lit::Char(lit_char) => {
                let c = lit_char.value();
                Ok(Regex::chars(CSet::singleton(c as i32)))
            }
            Lit::Int(lit_int) => match lit_int.base10_parse::<i32>() {
                Ok(c) => {
                    if c < 0 || c > CSet::max_code() {
                        return Err(syn::Error::new_spanned(
                            lit_int,
                            "Invalid Unicode code point: {c:0x4}",
                        ));
                    }
                    Ok(Regex::chars(CSet::singleton(c)))
                }
                Err(e) => Err(syn::Error::new_spanned(
                    lit_int,
                    format!(
                        "Expecting int in regex to be parsable as `i32` ({e}).",
                    ),
                )),
            },
            _ => Err(syn::Error::new_spanned(
                pat.clone(),
                "Invalid expression for a regex.",
            )),
        },
        Pat::Ident(PatIdent { ident, .. }) => {
            let name = ident.to_string();
            if let Some(c) = env.borrow().get(&name) {
                Ok(c.clone())
            } else if let Some(cset) = ferrelex_core::unicode_props::from_name(&name)
                .or_else(|| ferrelex_core::unicode_categories::from_name(&name))
            {
                Ok(Regex::chars(cset))
            } else {
                Err(syn::Error::new_spanned(
                    ident,
                    format!("Unbound regex: {name}."),
                ))
            }
        }
        _ => Err(syn::Error::new_spanned(
            pat,
            "Invalid expression for a regex",
        )),
    }
}
