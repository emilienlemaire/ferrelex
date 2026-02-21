use ferrelex_core::cset::CSet;
use quote::quote;
use std::rc::Rc;

use crate::table_name;

const LIMIT: isize = 8_192;

fn segments_of_partitions(partitions: &Vec<CSet>) -> Vec<(isize, isize, isize)> {
    let mut segments = vec![];
    partitions
        .into_iter()
        .cloned()
        .enumerate()
        .for_each(|(i, cset)| {
            let part: Vec<_> = cset.into();
            part.into_iter()
                .for_each(|(a, b)| segments.push((a, b, i as isize)));
        });
    segments.sort_by(|(a1, _, _), (a2, _, _)| a1.cmp(a2));
    segments
}

#[derive(Debug, Clone)]
pub(crate) enum DecisionTree {
    Lte(isize, Rc<DecisionTree>, Rc<DecisionTree>),
    Table(isize, Vec<isize>),
    Return(isize),
}

impl DecisionTree {
    pub fn simplify_decision_tree(self: Rc<Self>) -> Rc<Self> {
        use DecisionTree::*;
        match self.as_ref() {
            Table(_, _) | Return(_) => self.clone(),
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

    pub fn decision(l: &[(isize, isize, isize)]) -> Self {
        use DecisionTree::*;
        let l: Vec<_> = l
            .iter()
            .map(|(a, b, i)| (*a, *b, Rc::new(Return(*i))))
            .collect();

        fn merge2(
            l: Vec<(isize, isize, Rc<DecisionTree>)>,
        ) -> Vec<(isize, isize, Rc<DecisionTree>)> {
            let mut res = vec![];
            for elts in l.chunks(2) {
                let (a1, b1, d1) = &elts[0];
                let (a2, b2, d2) = &elts[1];
                let x = if b1 + 1 == *a2 {
                    d2.clone()
                } else {
                    Rc::new(Lte(a2 - 1, Rc::new(Return(-1)), d2.clone()))
                };
                res.push((*a1, *b2, Rc::new(Lte(*b1, d1.clone(), x.clone()))))
            }
            if l.len() % 2 == 1 {
                res.push(l[l.len() - 1].clone())
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

            if l.len() == 0 {
                break Return(-1);
            }

            l = merge2(l);
        }
    }

    fn _decision_table(l: Vec<(isize, isize, isize)>) -> Self {
        use DecisionTree::*;
        let split = l
            .iter()
            .position(|&(_, b, i)| b >= LIMIT || i >= 255)
            .unwrap_or(l.len());

        let table = &l[..split];
        let rest = &l[split..];

        match table {
            [] => Self::decision(&l),
            &[(min, max, i)] => Lte(
                min - 1,
                Rc::new(Return(-1)),
                Rc::new(Lte(max, Rc::new(Return(i)), Rc::new(Self::decision(rest)))),
            ),
            _ => {
                let min = table.iter().map(|&(a, _, _)| a).min().unwrap();
                let max = table.last().unwrap().1;

                let mut arr = vec![0isize; (max - min + 1) as usize];
                for &(a, b, i) in table {
                    for j in a..=b {
                        arr[(j - min) as usize] = i + 1;
                    }
                }

                Lte(
                    min - 1,
                    Rc::new(Return(-1)),
                    Rc::new(Lte(
                        max,
                        Rc::new(Table(min, arr)),
                        Rc::new(Self::decision(rest)),
                    )),
                )
            }
        }
    }

    pub fn simplify(self: Rc<Self>, min: isize, max: isize) -> Rc<Self> {
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

    pub fn decision_table(p: &Vec<CSet>) -> Rc<Self> {
        let decision_table = Rc::new(Self::_decision_table(segments_of_partitions(p)));
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
            Table(offset, t) => {
                let c = if *offset == 0 {
                    quote! {(c)}
                } else {
                    quote! {(c - #offset)}
                };
                let table_name = table_name(&t);
                quote! { (#table_name.chars().nth(#c as usize).expect("to be checked before") as isize - 1)}
            }
        }
    }
}
