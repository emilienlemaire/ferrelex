use std::{
    cell::{Cell, RefCell},
    hash::{Hash, Hasher},
    isize,
    rc::Rc,
    usize,
};

use ferrelex_core::cset::CSet;
use rustc_hash::FxHashMap;
use syn::{
    BinOp, Expr, ExprBinary, ExprCall, ExprLit, ExprParen, ExprPath, ExprRange, ExprTuple, Lit,
    Pat, PatIdent, PatLit, PatOr, PatParen, PatRange, PatTuple, PatTupleStruct, Path, RangeLimits,
    parse_quote,
};

use crate::Env;

thread_local! {
    static COUNTER: Cell<usize> = Cell::new(0);
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Node {
    id: usize,
    eps: Vec<NodeCell>,
    trans: Vec<(CSet, NodeCell)>,
}

#[derive(Clone, Debug)]
pub(crate) struct NodeCell(Rc<RefCell<Node>>);

impl Node {
    fn new() -> NodeCell {
        COUNTER.with(|c| {
            let v = c.get();
            c.set(v + 1);
            NodeCell(Rc::new(RefCell::new(Self {
                id: c.get(),
                eps: vec![],
                trans: vec![],
            })))
        })
    }
}

impl PartialEq for NodeCell {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for NodeCell {}

impl Hash for NodeCell {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // one usize = the heap address
        std::ptr::hash(Rc::as_ptr(&self.0), state);
    }
}

impl AsRef<Rc<RefCell<Node>>> for NodeCell {
    fn as_ref(&self) -> &Rc<RefCell<Node>> {
        &self.0
    }
}

/// Optional convenience:
impl std::ops::Deref for NodeCell {
    type Target = RefCell<Node>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<Rc<RefCell<Node>>> for NodeCell {
    fn from(rc: Rc<RefCell<Node>>) -> Self {
        Self(rc)
    }
}

pub(crate) type Regex = Rc<dyn Fn(NodeCell) -> NodeCell>;

pub(super) fn seq(r1: Regex, r2: Regex, succ: NodeCell) -> NodeCell {
    r1(r2(succ))
}

pub(super) fn is_chars(final_: NodeCell, node: NodeCell) -> Option<CSet> {
    if node.borrow().eps.is_empty() && node.borrow().trans.len() == 1 {
        if node.borrow().trans[0].1 == final_.into() {
            Some(node.borrow().trans[0].0.clone())
        } else {
            None
        }
    } else {
        None
    }
}

pub(super) fn chars(c: CSet, node: NodeCell) -> NodeCell {
    let n = Node::new();
    n.borrow_mut().trans.push((c, node));
    n
}

pub(super) fn alt(r1: Regex, r2: Regex, succ: NodeCell) -> NodeCell {
    let nr1 = r1(succ.clone());
    let nr2 = r2(succ.clone());
    match (
        is_chars(succ.clone(), nr1.clone()),
        is_chars(succ.clone(), nr2.clone()),
    ) {
        (Some(ref c1), Some(c2)) => chars(c1.clone().union(&c2), succ.clone()),
        _ => {
            let n = Node::new();
            n.borrow_mut().eps.extend(vec![nr1, nr2]);
            n
        }
    }
}

pub(super) fn rep(r: Regex, succ: NodeCell) -> NodeCell
where
{
    let n = Node::new();
    n.borrow_mut().eps.push(r(n.clone()));
    n.borrow_mut().eps.push(succ);
    n
}

pub(super) fn plus(r: Regex, succ: NodeCell) -> NodeCell {
    let n = Node::new();
    let nr = r(n.clone());
    n.borrow_mut().eps.push(nr.clone());
    n.borrow_mut().eps.push(succ);
    nr
}

pub(super) fn eps(succ: NodeCell) -> NodeCell {
    succ
}

pub(super) fn compl(r: Regex) -> Option<Regex>
where
{
    let n = Node::new();
    let r = r.clone();
    match is_chars(n.clone(), r(n.clone())) {
        Some(c) => Some(Rc::new(move |n| chars(CSet::any().difference(&c), n))),
        _ => None,
    }
}

pub(super) fn substract(r0: Regex, r1: Regex) -> Option<Regex>
where
{
    let n = Node::new();
    let to_chars = |r: Regex| is_chars(n.clone(), r(n.clone()));
    match (to_chars(r0), to_chars(r1)) {
        (Some(c1), Some(c2)) => Some(Rc::new(move |n| chars(c1.difference(&c2), n))),
        _ => None,
    }
}

pub(super) fn intersect(r0: Regex, r1: Regex) -> Option<Regex>
where
{
    let n = Node::new();
    let to_chars = |r: Regex| is_chars(n.clone(), r(n.clone()));
    match (to_chars(r0), to_chars(r1)) {
        (Some(c1), Some(c2)) => Some(Rc::new(move |n| chars(c1.intersection(&c2), n))),
        _ => None,
    }
}

pub(super) fn repeat(r: Regex, start: usize, end: usize) -> Regex {
    match (start, end) {
        (0, 0) => Rc::new(eps),
        (0, n) => {
            let mut res: Regex = Rc::new(eps.clone());
            for _ in 1..=n {
                let rc = r.clone();
                let new = Rc::new(move |n| {
                    let prev = res.clone();
                    let rc = rc.clone();
                    alt(
                        Rc::new(eps.clone()),
                        Rc::new(move |m| seq(rc.clone(), prev.clone(), m)),
                        n,
                    )
                });
                res = new;
            }
            res
        }
        (m, n) => {
            let mut res: Regex = r.clone();
            for _ in 1..m {
                let prev = res.clone();
                let rc = r.clone();
                let new = Rc::new(move |n| seq(rc.clone(), prev.clone(), n));
                res = new;
            }
            repeat(res, 0, n - m)
        }
    }
}

fn compile_re(re: Regex) -> (NodeCell, NodeCell)
where
{
    let final_ = Node::new();
    (re(final_.clone()), final_)
}

fn add_node(state: &mut Vec<NodeCell>, node: NodeCell) {
    if !state.iter().any(|n| Rc::ptr_eq(n.as_ref(), node.as_ref())) {
        state.push(node.clone());
        add_nodes(state, node.borrow().eps.clone())
    }
}

fn add_nodes(state: &mut Vec<NodeCell>, nodes: Vec<NodeCell>) {
    for n in nodes {
        add_node(state, n)
    }
}

fn transition(state: &Vec<NodeCell>) -> Vec<(CSet, Vec<NodeCell>)> {
    fn norm(l: &mut Vec<(CSet, NodeCell)>) {
        let mut idx = 0;
        while idx + 1 < l.len() {
            let (c1, n1) = l[idx].clone();
            let (c2, n2) = l[idx + 1].clone();

            if n1 == n2 {
                let union = c1.union(&c2);
                l[idx] = (union, n1);
                l.remove(idx + 1);
            } else {
                idx += 1
            }
        }
    }
    let mut t: Vec<_> = state
        .iter()
        .flat_map(|n| n.0.borrow().trans.clone())
        .collect();
    t.sort_by(|(_, n1), (_, n2)| (n1.borrow().id as isize - n2.borrow().id as isize).cmp(&0));
    norm(&mut t);

    fn split(
        elt: (CSet, Vec<(CSet, Vec<NodeCell>)>),
        other: (CSet, NodeCell),
    ) -> (CSet, Vec<(CSet, Vec<NodeCell>)>) {
        let (all, t) = elt;
        let (c0, n0) = other;
        let t = {
            let mut res = vec![(c0.difference(&all), vec![n0.clone()])];
            let v1: Vec<_> = t
                .clone()
                .iter()
                .map(|(c, ns)| {
                    let mut v = vec![n0.clone()];
                    v.extend(ns.clone());
                    (c.intersection(&c0), v)
                })
                .collect();
            let v2: Vec<_> = t
                .clone()
                .iter()
                .map(|(c, ns)| (c.difference(&c0), ns.clone()))
                .collect();
            res.extend(v1);
            res.extend(v2);
            res
        };
        (
            all.union(&c0),
            t.iter().filter(|(c, _)| !c.is_empty()).cloned().collect(),
        )
    }

    let (_, t) = t.into_iter().fold((CSet::new(), vec![]), split);
    let mut t: Vec<_> = t
        .into_iter()
        .map(|(c, ns)| {
            let mut v = vec![];
            add_nodes(&mut v, ns);
            (c, v)
        })
        .collect();
    t.sort_by(|(c1, _), (c2, _)| c1.cmp(c2));
    t
}

pub(super) fn compile(res: Vec<Regex>) -> Vec<(Vec<(CSet, isize)>, Vec<bool>)> {
    let rs: Vec<_> = res.clone().into_iter().map(compile_re).collect();
    let mut counter = 0;
    let mut states: FxHashMap<Vec<NodeCell>, usize> = FxHashMap::default();
    let mut states_def: FxHashMap<usize, (Vec<(CSet, isize)>, Vec<bool>)> = FxHashMap::default();

    fn aux(
        state: Vec<NodeCell>,
        counter: &mut usize,
        states: &mut FxHashMap<Vec<NodeCell>, usize>,
        states_def: &mut FxHashMap<usize, (Vec<(CSet, isize)>, Vec<bool>)>,
        rs: Vec<(NodeCell, NodeCell)>,
    ) -> isize {
        if let Some(id) = states.get(&state) {
            return (*id).try_into().unwrap();
        }

        let i = *counter;
        *counter += 1;
        states.insert(state.clone(), i);
        let mut trans = vec![];
        for (p, t) in transition(&state) {
            let target_id = aux(t, counter, states, states_def, rs.clone());
            trans.push((p, target_id));
        }
        let finals: Vec<_> = rs
            .iter()
            .map(|(_, f)| state.iter().any(|s| Rc::ptr_eq(s.as_ref(), f.as_ref())))
            .collect();
        states_def.insert(i, (trans, finals));
        i.try_into().unwrap()
    }

    let mut init = vec![];
    for (i, _) in &rs {
        add_node(&mut init, i.clone())
    }
    let i = aux(init, &mut counter, &mut states, &mut states_def, rs);
    assert_eq!(i, 0);
    (0..counter)
        .map(|i| states_def.remove(&i).expect("missing state"))
        .collect()
}

pub(super) fn builtin_regex() -> Env {
    let mut regex: FxHashMap<String, Regex> = FxHashMap::default();
    regex.insert("any".into(), Rc::new(|n| chars(CSet::any(), n)));
    regex.insert("eof".into(), Rc::new(|n| chars(CSet::eof(), n)));
    regex
}

pub(super) fn regex_of_expr<'a>(env: Rc<RefCell<Env>>, expr: Expr) -> Result<Regex, syn::Error> {
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
            Ok(Rc::new(move |n| alt(r1.clone(), r2.clone(), n)))
        }
        Expr::Tuple(ExprTuple { elems, .. }) => match elems.clone().first() {
            Some(elem) => elems.into_iter().skip(1).fold(
                regex_of_expr(env.clone(), elem.clone()),
                |acc, elem| {
                    let regex = regex_of_expr(env.clone(), elem)?;
                    let acc = acc?;
                    Ok(Rc::new(move |n| seq(acc.clone(), regex.clone(), n)))
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
                    Ok(Rc::new(move |n| rep(arg_regex.clone(), n)))
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
                    Ok(Rc::new(move |n| plus(args_regex.clone(), n)))
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
                                            repeat(regex, start, end - 1)
                                        } else {
                                            repeat(regex, start, end)
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
                                Ok(repeat(regex, v, v))
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
                    Ok(Rc::new(move |n| alt(Rc::new(eps), regex.clone(), n)))
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
                    match compl(regex) {
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
                    match substract(r1, r2) {
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
                    match intersect(r1, r2) {
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
                            Ok(Rc::new(move |n| chars(c.clone(), n)))
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
                    if c1.len_utf8() > 1 || c2.len_utf8() > 1 {
                        return Err(syn::Error::new_spanned(
                            expr.clone(),
                            "A character range expect only one byte characters as bounds.",
                        ));
                    }
                    let i1 = c1 as u8 as isize;
                    let i2 = c2 as u8 as isize
                        - if let RangeLimits::HalfOpen(_) = limits {
                            1
                        } else {
                            0
                        };
                    let set = CSet::interval(i1, i2);
                    Ok(Rc::new(move |n| chars(set.clone(), n)))
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
                        let i2 = i2
                            - if let RangeLimits::HalfOpen(_) = limits {
                                1
                            } else {
                                0
                            };
                        Ok(Rc::new(move |n| chars(CSet::interval(i1, i2), n)))
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
                .bytes()
                .map(|i| CSet::singleton(i.into()))
                .fold(Rc::new(eps), |acc, s| {
                    let c = Rc::new(move |n| chars(s.clone(), n));
                    Rc::new(move |n| seq(acc.clone(), c.clone(), n))
                })),
            Lit::Char(lit_char) => {
                if lit_char.value().len_utf8() > 1 {
                    return Err(syn::Error::new_spanned(
                        lit_char,
                        "Expecting chars of one code point in regex.",
                    ));
                }
                let c = lit_char.value();
                Ok(Rc::new(move |n| chars(CSet::singleton(c as isize), n)))
            }
            Lit::Int(lit_int) => match lit_int.base10_parse::<isize>() {
                Ok(c) => {
                    if c < 0 || c > CSet::max_code() {
                        return Err(syn::Error::new_spanned(
                            lit_int,
                            "Invalid Unicode code point: {c:0x4}",
                        ));
                    }
                    Ok(Rc::new(move |n| chars(CSet::singleton(c), n)))
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
            match env.borrow().get(&name) {
                Some(c) => Ok(c.clone()),
                None => Err(syn::Error::new_spanned(
                    segments,
                    format!("Unbound regex: {name}."),
                )),
            }
        }
        _ => Err(syn::Error::new_spanned(
            expr,
            "Invalid expression for a regex",
        )),
    }
}

pub(super) fn regex_of_pattern<'a>(env: Rc<RefCell<Env>>, pat: Pat) -> Result<Regex, syn::Error> {
    match pat.clone() {
        Pat::Paren(PatParen { pat, .. }) => regex_of_pattern(env.clone(), *pat),
        Pat::Or(PatOr { cases, .. }) => {
            let mut regex = regex_of_pattern(env.clone(), cases[0].clone())?;
            for pat in cases.iter().skip(1) {
                let new_regex = regex_of_pattern(env.clone(), pat.clone())?;
                regex = Rc::new(move |n| alt(regex.clone(), new_regex.clone(), n));
            }
            Ok(regex)
        }
        Pat::Tuple(PatTuple { elems, .. }) => match elems.clone().first() {
            Some(elem) => elems.into_iter().skip(1).fold(
                regex_of_pattern(env.clone(), elem.clone()),
                |acc, elem| {
                    let regex = regex_of_pattern(env.clone(), elem)?;
                    let acc = acc?;
                    Ok(Rc::new(move |n| seq(acc.clone(), regex.clone(), n)))
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
                    Ok(Rc::new(move |n| rep(arg_regex.clone(), n)))
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
                    Ok(Rc::new(move |n| plus(elems_regex.clone(), n)))
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
                                            repeat(regex, start, end - 1)
                                        } else {
                                            repeat(regex, start, end)
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
                                Ok(repeat(regex, v, v))
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
                    Ok(Rc::new(move |n| alt(Rc::new(eps), regex.clone(), n)))
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
                    match compl(regex) {
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
                    match substract(r1, r2) {
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
                    match intersect(r1, r2) {
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
                            Ok(Rc::new(move |n| chars(c.clone(), n)))
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
                    if c1.len_utf8() > 1 || c2.len_utf8() > 1 {
                        return Err(syn::Error::new_spanned(
                            pat.clone(),
                            "A character range expect only one byte characters as bounds.",
                        ));
                    }
                    let i1 = c1 as u8 as isize;
                    let i2 = c2 as u8 as isize
                        - if let RangeLimits::HalfOpen(_) = limits {
                            1
                        } else {
                            0
                        };
                    let set = CSet::interval(i1, i2);
                    Ok(Rc::new(move |n| chars(set.clone(), n)))
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
                        let i2 = i2
                            - if let RangeLimits::HalfOpen(_) = limits {
                                1
                            } else {
                                0
                            };
                        Ok(Rc::new(move |n| chars(CSet::interval(i1, i2), n)))
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
                .fold(Rc::new(eps), |acc, s| {
                    let c = Rc::new(move |n| chars(s.clone(), n));
                    Rc::new(move |n| seq(acc.clone(), c.clone(), n))
                })),
            Lit::Char(lit_char) => {
                if lit_char.value().len_utf8() > 1 {
                    return Err(syn::Error::new_spanned(
                        lit_char,
                        "Expecting chars of one code point in regex.",
                    ));
                }
                let c = lit_char.value();
                Ok(Rc::new(move |n| chars(CSet::singleton(c as isize), n)))
            }
            Lit::Int(lit_int) => match lit_int.base10_parse::<isize>() {
                Ok(c) => {
                    if c < 0 || c > CSet::max_code() {
                        return Err(syn::Error::new_spanned(
                            lit_int,
                            "Invalid Unicode code point: {c:0x4}",
                        ));
                    }
                    Ok(Rc::new(move |n| chars(CSet::singleton(c), n)))
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
            match env.borrow().get(&name) {
                Some(c) => Ok(c.clone()),
                None => Err(syn::Error::new_spanned(
                    ident,
                    format!("Unbound regex: {name}."),
                )),
            }
        }
        _ => Err(syn::Error::new_spanned(
            pat,
            "Invalid expression for a regex",
        )),
    }
}
