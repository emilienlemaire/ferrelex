use std::{cell::RefCell, rc::Rc};

use ferrelex_core::cset::CSet;
use rustc_hash::FxHashMap;
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
        Self::Seq(Box::new(self), Box::new(r))
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

    pub(crate) fn substract(&self, r: &Self) -> Option<Self> {
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
                let mut res = self.clone();
                for _ in 1..m {
                    let new = self.clone().seq(res);
                    res = new;
                }
                res.repeat(0, n - m)
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

    fn add_node(&self, state: &mut Vec<NodeId>, id: NodeId) {
        match state.binary_search(&id) {
            Ok(_) => return,
            Err(pos) => state.insert(pos, id),
        }

        for &eps_id in &self.0[id].eps {
            self.add_node(state, eps_id);
        }
    }

    fn add_nodes(&self, ids: &[NodeId]) -> Vec<NodeId> {
        let mut state = vec![];
        for &id in ids {
            self.add_node(&mut state, id);
        }
        state
    }

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

pub(super) fn compile(regexs: &[Regex]) -> Vec<(Vec<(CSet, isize)>, Vec<bool>)> {
    let mut nfa = Nfa::new();

    let compiled: Vec<(NodeId, NodeId)> = regexs.iter().map(|re| nfa.compile_re(re)).collect();

    let mut state_map: FxHashMap<Vec<NodeId>, isize> = Default::default();
    let mut worklist: Vec<Vec<NodeId>> = vec![];
    let mut states_def: Vec<(Vec<(CSet, isize)>, Vec<bool>)> = vec![];

    let intern = |state: Vec<NodeId>,
                  worklist: &mut Vec<Vec<NodeId>>,
                  states_def: &mut Vec<(Vec<(CSet, isize)>, Vec<bool>)>,
                  state_map: &mut FxHashMap<Vec<NodeId>, isize>|
     -> isize {
        let next = state_map.len() as isize;
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
            .map(|(_, f)| worklist[cursor].contains(f))
            .collect();
        states_def[cursor] = (trans, finals);
        cursor += 1;
    }

    states_def
}

pub(super) fn builtin_regex() -> Env {
    let mut regex: FxHashMap<String, Regex> = FxHashMap::default();
    regex.insert("any".into(), Regex::chars(CSet::any()));
    regex.insert("eof".into(), Regex::chars(CSet::eof()));
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
                                        let err_string = e.to_string();
                                        Err(syn::Error::new_spanned(
                                            args[1].clone(),
                                            format!(
                                                "Could not parse integer literal as `usize` value: {}",
                                                &err_string
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
                                let err_string = e.to_string();
                                Err(syn::Error::new_spanned(
                                    args[1].clone(),
                                    format!(
                                        "Could not parse integer literal as `usize` value: {}",
                                        &err_string
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
                            "`Sub` operator is expecting two arguements",
                        ));
                    }
                    let r1 = regex_of_expr(env.clone(), args[0].clone())?;
                    let r2 = regex_of_expr(env.clone(), args[1].clone())?;
                    match r1.substract(&r2) {
                        Some(r) => Ok(r),
                        None => Err(syn::Error::new_spanned(
                            args,
                            "`Sub` operator can only be applied to single character length regexs",
                        )),
                    }
                }
                e if e == parse_quote! {Intersect} => {
                    if args.len() != 2 {
                        return Err(syn::Error::new_spanned(
                            args.clone(),
                            "`Intersect` operator is expecting two arguements",
                        ));
                    }
                    let r1 = regex_of_expr(env.clone(), args[0].clone())?;
                    let r2 = regex_of_expr(env.clone(), args[1].clone())?;
                    match r1.intersect(&r2) {
                        Some(r) => Ok(r),
                        None => Err(syn::Error::new_spanned(
                            args,
                            "`Intersect` operator can only be applied to signle character length regexs",
                        )),
                    }
                }
                e if e == parse_quote! {Chars} => {
                    if args.len() > 1 {
                        return Err(syn::Error::new_spanned(
                            args,
                            "`Chars` operator only accepts one arguement.",
                        ));
                    }
                    match &args[0] {
                        Expr::Lit(ExprLit {
                            lit: Lit::Str(lit_str),
                            ..
                        }) => {
                            let string = lit_str.value();
                            let c = string
                                .bytes()
                                .map(|b| CSet::singleton(b.into()))
                                .fold(CSet::new(), |acc, s| acc.union(&s));
                            Ok(Regex::chars(c))
                        }
                        _ => Err(syn::Error::new_spanned(
                            args,
                            "`Chars` operator only accepts str literal as argument.",
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
            start, limits: _, end, ..
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
                    if c1.len_utf8() > 1 || c2.len_utf8() > 1 {
                        return Err(syn::Error::new_spanned(
                            expr.clone(),
                            "A character range expect only one byte characters as bounds.",
                        ));
                    }
                    let i1 = c1 as u8 as isize;
                    let i2 = c2 as u8 as isize;
                    let set = CSet::interval(i1, i2);
                    Ok(Regex::chars(set.clone()))
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
                ) => match (
                    i1_lit.base10_parse::<isize>(),
                    i2_lit.base10_parse::<isize>(),
                ) {
                    (Ok(i1), Ok(i2)) => {
                        if i1 < 0 || i1 > CSet::max_code() {
                            return Err(syn::Error::new_spanned(
                                i1_lit.clone(),
                                "Invalid Unicode character code: {i1:0x4}",
                            ));
                        }
                        if i2 < 0 || i2 > CSet::max_code() {
                            return Err(syn::Error::new_spanned(
                                i2_lit.clone(),
                                "Invalid Unicode character code: {i2:0x4}",
                            ));
                        }
                        Ok(Regex::chars(CSet::interval(i1, i2)))
                    }
                    (Err(e), _) | (_, Err(e)) => Err(syn::Error::new_spanned(
                        expr,
                        format!(
                            "An integer range should have integer parsable as `isize`: {}",
                            e.to_string()
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
                .map(|i| CSet::singleton(i as isize))
                .fold(Regex::Eps, |acc, s| {
                    let c = Regex::chars(s);
                    acc.seq(c)
                })),
            Lit::Char(lit_char) => {
                let c = lit_char.value();
                Ok(Regex::chars(CSet::singleton(c as isize)))
            }
            Lit::Int(lit_int) => match lit_int.base10_parse::<isize>() {
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
                        "Expecting int in regex to be parsable as `isize` ({}).",
                        e.to_string()
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
                                        let err_string = e.to_string();
                                        Err(syn::Error::new_spanned(
                                            elems[1].clone(),
                                            format!(
                                                "Could not parse integer literal as `usize` value: {}",
                                                &err_string
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
                                let err_string = e.to_string();
                                Err(syn::Error::new_spanned(
                                    elems[1].clone(),
                                    format!(
                                        "Could not parse integer literal as `usize` value: {}",
                                        &err_string
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
                            "`Sub` operator is expecting two arguements",
                        ));
                    }
                    let r1 = regex_of_pattern(env.clone(), elems[0].clone())?;
                    let r2 = regex_of_pattern(env.clone(), elems[1].clone())?;
                    match r1.substract(&r2) {
                        Some(r) => Ok(r),
                        None => Err(syn::Error::new_spanned(
                            elems,
                            "`Sub` operator can only be applied to single character length regexs",
                        )),
                    }
                }
                e if e == parse_quote! {Intersect} => {
                    if elems.len() != 2 {
                        return Err(syn::Error::new_spanned(
                            elems.clone(),
                            "`Intersect` operator is expecting two arguements",
                        ));
                    }
                    let r1 = regex_of_pattern(env.clone(), elems[0].clone())?;
                    let r2 = regex_of_pattern(env.clone(), elems[1].clone())?;
                    match r1.intersect(&r2) {
                        Some(r) => Ok(r),
                        None => Err(syn::Error::new_spanned(
                            elems,
                            "`Intersect` operator can only be applied to signle character length regexs",
                        )),
                    }
                }
                e if e == parse_quote! {Chars} => {
                    if elems.len() > 1 {
                        return Err(syn::Error::new_spanned(
                            elems,
                            "`Chars` operator only accepts one arguement.",
                        ));
                    }
                    match &elems[0] {
                        Pat::Lit(PatLit {
                            lit: Lit::Str(lit_str),
                            ..
                        }) => {
                            let string = lit_str.value();
                            let c = string
                                .bytes()
                                .map(|b| CSet::singleton(b.into()))
                                .fold(CSet::new(), |acc, s| acc.union(&s));
                            Ok(Regex::chars(c))
                        }
                        _ => Err(syn::Error::new_spanned(
                            elems,
                            "`Chars` operator only accepts str literal as argument.",
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
            start, limits: _, end, ..
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
                    if c1.len_utf8() > 1 || c2.len_utf8() > 1 {
                        return Err(syn::Error::new_spanned(
                            pat.clone(),
                            "A character range expect only one byte characters as bounds.",
                        ));
                    }
                    let i1 = c1 as u8 as isize;
                    let i2 = c2 as u8 as isize;
                    let set = CSet::interval(i1, i2);
                    Ok(Regex::chars(set))
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
                ) => match (
                    i1_lit.base10_parse::<isize>(),
                    i2_lit.base10_parse::<isize>(),
                ) {
                    (Ok(i1), Ok(i2)) => {
                        if i1 < 0 || i1 > CSet::max_code() {
                            return Err(syn::Error::new_spanned(
                                i1_lit.clone(),
                                "Invalid Unicode character code: {i1:0x4}",
                            ));
                        }
                        if i2 < 0 || i2 > CSet::max_code() {
                            return Err(syn::Error::new_spanned(
                                i2_lit.clone(),
                                "Invalid Unicode character code: {i2:0x4}",
                            ));
                        }
                        Ok(Regex::chars(CSet::interval(i1, i2)))
                    }
                    (Err(e), _) | (_, Err(e)) => Err(syn::Error::new_spanned(
                        pat,
                        format!(
                            "An integer range should have integer parsable as `isize`: {}",
                            e.to_string()
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
                .bytes()
                .map(|i| CSet::singleton(i.into()))
                .fold(Regex::eps(), |acc, s| {
                    let c = Regex::chars(s);
                    acc.seq(c)
                })),
            Lit::Char(lit_char) => {
                if lit_char.value().len_utf8() > 1 {
                    return Err(syn::Error::new_spanned(
                        lit_char,
                        "Expecting chars of one code point in regex.",
                    ));
                }
                let c = lit_char.value();
                Ok(Regex::chars(CSet::singleton(c as isize)))
            }
            Lit::Int(lit_int) => match lit_int.base10_parse::<isize>() {
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
                        "Expecting int in regex to be parsable as `isize` ({}).",
                        e.to_string()
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
