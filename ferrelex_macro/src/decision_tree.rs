use ferrelex_core::cset::CSet;
use quote::quote;

use crate::table_name;

const LIMIT: isize = 8_192;

fn segments_of_partitions(partitions: &Vec<CSet>) -> Vec<(isize, isize, isize)> {
    let mut segments = vec![];
    partitions.into_iter().cloned().enumerate().for_each(|(i, cset)| {
        let part: Vec<_> = cset.into();
        part.into_iter().for_each(|(a, b)| segments.push((a, b, i as isize)));
    });
    segments.sort_by(|(a1, _, _), (a2, _, _)| a1.cmp(a2));
    segments
}

#[derive(Debug, Clone)]
pub(crate) enum DecisionTree {
    Lte(isize, Box<DecisionTree>, Box<DecisionTree>),
    Table(isize, Vec<isize>),
    Return(isize),
}

impl DecisionTree {
    pub fn simplify_decision_tree(self) -> Self {
        use DecisionTree::*;
        match self {
            Table(_, _) | Return(_) => self,
            Lte(i, l, r) => match (&*l, &*r) {
                (Return(a), Return(b)) if a == b => *l,
                _ => {
                    let l = l.simplify_decision_tree();
                    let r = r.simplify_decision_tree();
                    match (&l, &r) {
                        (Return(a), Return(b)) if a == b => l,
                        _ => Lte(i, Box::new(l), Box::new(r)),
                    }
                }
            },
        }
    }

    pub fn decision(l: Vec<(isize, isize, isize)>) -> Self {
        use DecisionTree::*;
        let l: Vec<_> = l.into_iter().map(|(a, b, i)| (a, b, Return(i))).collect();

        fn merge2(l: Vec<(isize, isize, DecisionTree)>) -> Vec<(isize, isize, DecisionTree)> {
            let mut res = vec![];
            for elts in l.chunks(2) {
                let (a1, b1, d1) = &elts[0];
                let (a2, b2, d2) = &elts[1];
                let x = if b1 + 1 == *a2 {
                    d1.clone()
                } else {
                    Lte(a2 - 1, Box::new(Return(-1)), Box::new(d2.clone()))
                };
                res.push((*a1, *b2, Lte(*b1, Box::new(d1.clone()), Box::new(x))))
            }
            if l.len() % 2 == 1 {
                res.push(l[l.len() - 1].clone())
            }
            res
        }

        let mut l = merge2(l);

        while l.len() > 1 {
            l = merge2(l);
        }

        if l.len() == 0 {
            Return(-1)
        } else {
            let (a, b, d) = l[0].clone();
            Lte(
                a - 1,
                Box::new(Return(-1)),
                Box::new(Lte(b, Box::new(d), Box::new(Return(-1)))),
            )
        }
    }

    fn _decision_table(l: Vec<(isize, isize, isize)>) -> Self {
        use DecisionTree::*;
        fn aux(
            l: &[(isize, isize, isize)],
        ) -> (isize, Vec<(isize, isize, isize)>, &[(isize, isize, isize)]) {
            let mut min = isize::MAX;
            let mut accu = vec![];
            let mut idx = 0usize;

            while let Some((a, b, i)) = l.get(idx) {
                if *b < LIMIT && *i < 255 {
                    min = (*a).min(min);
                    accu.push((*a, *b, *i));
                    idx += 1;
                } else {
                    break;
                }
            }
            accu.reverse();
            (min, accu, &l[idx..])
        }
        let (min, table, rest) = aux(&l);

        if table.len() == 0 {
            Self::decision(l)
        } else if table.len() == 1 {
            let (min, max, i) = table[0];
            Lte(
                min - 1,
                Box::new(Return(-1)),
                Box::new(Lte(
                    max,
                    Box::new(Return(i)),
                    Box::new(Self::decision(Vec::from(rest))),
                )),
            )
        } else {
            let (_, max, _) = table[0];
            let mut v = vec![0; (max - min + 1) as usize];
            table.iter().for_each(|(a, b, i)| {
                for j in *a..=*b {
                    v[(j - min) as usize] = i + 1;
                }
            });
            Lte(
                min - 1,
                Box::new(Return(-1)),
                Box::new(Lte(
                    max,
                    Box::new(Table(min, v)),
                    Box::new(Self::decision(Vec::from(rest))),
                )),
            )
        }
    }

    pub fn simplify(self, min: isize, max: isize) -> Self {
        use DecisionTree::*;
        match self {
            Lte(i, yes, no) => {
                if i >= max {
                    yes.simplify(min, max)
                } else if i < min {
                    no.simplify(min, max)
                } else {
                    let simpl_yes = yes.simplify(min, i);
                    let simpl_no = no.simplify(i + 1, max);
                    Lte(i, Box::new(simpl_yes), Box::new(simpl_no))
                }
            }
            x => x
        }
    }

    pub fn decision_table(p: &Vec<CSet>) -> Self {
        let decision_table = Self::_decision_table(segments_of_partitions(p));
        decision_table.simplify(-1, CSet::max_code())
    }

    pub fn gen_tokens(self) -> proc_macro2::TokenStream {
        use DecisionTree::*;
        match self {
            Lte(i, yes, no) => {
                let yes_toks = yes.gen_tokens();
                let no_toks = no.gen_tokens();
                quote! {
                    if c < #i {
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
                let c = if offset == 0 {
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
