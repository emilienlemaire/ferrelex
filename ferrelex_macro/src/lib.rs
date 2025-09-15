use decision_tree::DecisionTree;
use ferrelex_core::cset::CSet;
use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use regex::{Regex, compile, regex_of_expr, regex_of_pattern};
use rustc_hash::FxHashMap;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        LazyLock, Mutex,
        atomic::{AtomicIsize, Ordering},
    },
};
use syn::{
    Arm, Attribute, Expr, ExprMatch, ExprPath, File, Ident, ItemFn, Pat, Stmt,
    Type, TypePath, parse_macro_input, parse_quote,
};

mod decision_tree;
mod regex;

pub(crate) type Env = FxHashMap<String, Regex>;

static PARTITIONS: LazyLock<Mutex<FxHashMap<Vec<CSet>, String>>> =
    LazyLock::new(|| Mutex::new(FxHashMap::default()));

static PARTITION_COUNTER: LazyLock<AtomicIsize> = LazyLock::new(|| AtomicIsize::new(0));

static TABLES: LazyLock<Mutex<FxHashMap<Vec<isize>, String>>> =
    LazyLock::new(|| Mutex::new(FxHashMap::default()));

static TABLE_COUNTER: LazyLock<AtomicIsize> = LazyLock::new(|| AtomicIsize::new(0));

fn best_final(finals: &Vec<bool>) -> Option<isize> {
    let mut fin = None;
    for i in finals.len() - 1..=0 {
        if finals[i] {
            fin = Some(i as isize)
        }
    }
    fin
}

fn appfun(fun: &Ident, args: Vec<TokenStream>) -> TokenStream {
    let args: Vec<proc_macro2::TokenStream> = args
        .into_iter()
        .map(proc_macro2::TokenStream::from)
        .collect();
    quote! { #fun(#(#args),*)}.into()
}

fn state_fun(state: isize) -> Ident {
    format_ident!("__ferrelex_state_{state}")
}

fn call_state(
    lexbuf: &Ident,
    auto: &Vec<(Vec<(CSet, isize)>, Vec<bool>)>,
    state: isize,
) -> TokenStream {
    let (trans, final_) = &auto[state as usize];
    if trans.is_empty() {
        let i = best_final(final_).expect("to be present") as isize;
        quote! {#i}.into()
    } else {
        appfun(&state_fun(state), vec![lexbuf.into_token_stream().into()])
    }
}

fn get_partitions() -> Vec<(String, Vec<CSet>)> {
    let partitions = PARTITIONS.lock().expect("to be lockable");
    partitions
        .iter()
        .map(|(k, v)| (v.clone(), k.clone()))
        .collect()
}

fn partition_name(p: Vec<CSet>) -> Ident {
    let mut partitions = PARTITIONS.lock().expect("to not be locked.");
    match partitions.get(&p) {
        None => {
            let mut c = PARTITION_COUNTER.fetch_add(1, Ordering::Relaxed);
            c += 1;
            let name = format!("__ferrelex_partition_{c}");
            partitions.insert(p.clone(), name.clone());
            format_ident!("{name}")
        }
        Some(n) => {
            format_ident!("{n}")
        }
    }
}

fn partition(name: &String, p: &Vec<CSet>) -> proc_macro2::TokenStream {
    let body = DecisionTree::decision_table(p)
        .simplify_decision_tree()
        .gen_tokens();
    let name = format_ident!("{name}");
    quote! {
        fn #name(c: isize) -> isize {
            #body
        }
    }
    .into()
}

pub(crate) fn table_name(t: &Vec<isize>) -> Ident {
    let mut tables = TABLES.lock().expect("to not be locked.");
    match tables.get(t) {
        None => {
            let mut c = TABLE_COUNTER.fetch_add(1, Ordering::Relaxed);
            c += 1;
            let name = format!("__ferrelex_table_{c}");
            tables.insert(t.clone(), name.clone());
            format_ident!("{name}")
        }
        Some(n) => {
            format_ident!("{n}")
        }
    }
}

fn table(name: &String, t: &Vec<isize>) -> proc_macro2::TokenStream {
    let s: String = t
        .into_iter()
        .map(|i| *i as u8)
        .collect::<Vec<_>>()
        .into_iter()
        .map(|u| u as char)
        .collect();
    let name = format_ident!("{name}");
    quote! { static #name: &str = #s; }
}

fn get_tables() -> Vec<(String, Vec<isize>)> {
    let tables = TABLES.lock().expect("to be lockable");
    tables.iter().map(|(k, v)| (v.clone(), k.clone())).collect()
}

fn gen_state(
    lexbuf: &Ident,
    auto: &Vec<(Vec<(CSet, isize)>, Vec<bool>)>,
    i: isize,
    elt: (Vec<(CSet, isize)>, Vec<bool>),
) -> TokenStream {
    let (trans, final_) = elt;
    let (partition, _): (Vec<_>, Vec<_>) = trans.clone().into_iter().unzip();
    let cases: Vec<_> = trans
        .clone()
        .into_iter()
        .enumerate()
        .map(|(i, (_, j))| {
            let e: proc_macro2::TokenStream = call_state(lexbuf, auto, j).into();
            let i_isize = i as isize;
            quote! { #i_isize => { #e } }
        })
        .collect();
    let body: proc_macro2::TokenStream = {
        let matched_expr: proc_macro2::TokenStream = appfun(
            &partition_name(partition),
            vec![quote! { #lexbuf.next_int() }.into()],
        )
        .into();
        let mut cases = cases.clone();
        cases.push(quote! { _ => { #lexbuf.backtrack() }});
        quote! {
            match #matched_expr {
                #(#cases),*
            }
        }
    };
    fn ret(body: proc_macro2::TokenStream, lexbuf: &Ident, i: isize) -> TokenStream {
        let name = state_fun(i);
        quote! {fn #name(#lexbuf: ferrelex::Lexbuf) -> isize {
            #body
        }}
        .into()
    }
    match best_final(&final_) {
        None => ret(body.clone(), lexbuf, i),
        Some(_) if trans.len() == 0 => quote! {}.into(),
        Some(i) => {
            let body = body.clone();
            ret(
                quote! {
                    #lexbuf.mark(#i);
                    #body
                }
                .into(),
                lexbuf,
                i,
            )
        }
    }
}

fn gen_definition(lexbuf: &Ident, l: Vec<(Regex, TokenStream)>, error: TokenStream) -> TokenStream {
    let compiled_regex = {
        let (fst, _): (Vec<_>, Vec<_>) = l.clone().into_iter().unzip();
        compile(fst)
    };
    let mut cases: Vec<proc_macro2::TokenStream> = l
        .iter()
        .enumerate()
        .map(|(i, (_, e))| {
            let e: proc_macro2::TokenStream = e.clone().into();
            let i_isize = i as isize;
            quote! { #i_isize => { #e } }.into()
        })
        .collect();
    let states: Vec<proc_macro2::TokenStream> = compiled_regex
        .iter()
        .enumerate()
        .map(|(i, elt)| gen_state(lexbuf, &compiled_regex, i as isize, elt.clone()).into())
        .collect();
    let state_0: proc_macro2::TokenStream =
        appfun(&state_fun(0), vec![quote! {#lexbuf}.into()]).into();
    let error: proc_macro2::TokenStream = error.into();
    cases.push(quote! {_ => #error}.into());
    quote! {
        #(#states)*
        #lexbuf.start();
        match #state_0 {
            #(#cases),*
        }
    }
    .into()
}

fn expression(env: Rc<RefCell<Env>>, expr: &Expr) -> TokenStream {
    match expr {
        Expr::Match(expr_match) => match expr_match {
            ExprMatch {
                expr, arms, attrs, ..
            } => {
                if attrs.contains((&parse_quote! {#[lexer]}) as &Attribute) {
                    match *expr.clone() {
                        Expr::Path(ExprPath {
                            qself: None, path, ..
                        }) => {
                            if path.segments.len() != 1 {
                                return syn::Error::new_spanned(
                                    expr,
                                    "expecting only an identifier to matched against \
                                    regexs.",
                                )
                                .to_compile_error()
                                .into();
                            }
                            let lexbuf = &path.segments[0].ident.clone();
                            let error = match arms.last() {
                                Some(Arm {
                                    pat: Pat::Wild(_),
                                    body: expr,
                                    guard: None,
                                    ..
                                }) => expression(env.clone(), expr.as_ref()),
                                _ => {
                                    return syn::Error::new_spanned(
                                        expr.clone(),
                                        "expecting a wildcard for error handling as last \
                                    match arm.",
                                    )
                                    .to_compile_error()
                                    .into();
                                }
                            };

                            let cases: Result<Vec<(Regex, TokenStream)>, _> = arms
                                [0..arms.len() - 1]
                                .iter()
                                .map(|arm| match arm {
                                    Arm { guard: Some(_), .. } => {
                                        Err(syn::Error::new_spanned(arm, "guard are not supported"))
                                    }
                                    Arm { pat, body, .. } => Ok((
                                        regex_of_pattern(env.clone(), pat.clone())?,
                                        expression(env.clone(), body.as_ref()),
                                    )),
                                })
                                .collect();
                            match cases {
                                Ok(cases) => gen_definition(lexbuf, cases, error),
                                Err(e) => return e.to_compile_error().into(),
                            }
                        }
                        _ => {
                            return syn::Error::new_spanned(
                                expr,
                                "expecting only an identifier to matched against regexs.",
                            )
                            .to_compile_error()
                            .into();
                        }
                    }
                } else {
                    expr.to_token_stream().into()
                }
            }
        },
        _ => expr.to_token_stream().into(),
    }
}

#[proc_macro]
pub fn lex(input: TokenStream) -> TokenStream {
    let f = parse_macro_input!(input as File);
    let env = Rc::new(RefCell::new(regex::builtin_regex()));
    let mut res = quote! {};
    let regex_typ: TypePath = syn::parse_quote! {Regex};
    for item in f.items {
        match item {
            syn::Item::Const(item_const) => match *item_const.ty {
                Type::Path(path) if path == regex_typ => {
                    let name = item_const.ident.to_string();
                    let regex = match regex_of_expr(env.clone(), *item_const.expr) {
                        Ok(v) => v,
                        Err(e) => return e.to_compile_error().into(),
                    };
                    env.borrow_mut().insert(name, regex);
                }
                _ => {
                    return syn::Error::new_spanned(
                        &item_const,
                        "expecting only `Regex` typed const.",
                    )
                    .to_compile_error()
                    .into();
                }
            },
            syn::Item::Fn(ItemFn {
                vis,
                sig,
                attrs,
                block,
            }) => {
                let mut new_block = quote! {};
                block.stmts.iter().for_each(|stmt| match stmt {
                    Stmt::Expr(e, semi) => {
                        let e = expression(env.clone(), e);
                        new_block.extend(proc_macro2::TokenStream::from(e));
                        if let Some(semi) = semi {
                            new_block.extend(semi.into_token_stream());
                        }
                    }
                    stmt => new_block.extend(stmt.into_token_stream()),
                });
                let new_fun = ItemFn {
                    vis,
                    sig,
                    attrs,
                    block: Box::new(parse_quote! { { #new_block } }),
                };
                res.extend(new_fun.into_token_stream());
            }
            other => {
                return syn::Error::new_spanned(
                    &other,
                    "expecting only `const` and `fn` items in the `lex` macro",
                )
                .to_compile_error()
                .into();
            }
        }
    }
    let partitions: Vec<_> = get_partitions()
        .iter()
        .map(|(name, p)| partition(name, p))
        .collect();
    let tables: Vec<_> = get_tables()
        .iter()
        .map(|(name, t)| table(name, t))
        .collect();
    quote! {
        #(#tables)*
        #(#partitions)*
        #res
    }
    .into()
}

#[cfg(test)]
mod tests {
static __ferrelex_table_3: &str = "\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\0\0\0\0\0\0\0\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}";
fn __ferrelex_partition_2(c: isize) -> isize {
    -1isize
}
fn __ferrelex_partition_1(c: isize) -> isize {
    if c < 64isize {
        -1isize
    } else {
        if c < 121isize {
            (__ferrelex_table_3
                .chars()
                .nth((c - 65isize))
                .expect("to be checked before")
                .into() - 1)
        } else {
            -1isize
        }
    }
}
pub fn lex(lexbuf: ferrelex::Lexbuf) {
    fn __ferrelex_state_0(lexbuf: ferrelex::LexBuf) -> isize {
        match __ferrelex_partition_1(lexbuf.next_int()) {
            0isize => 0isize,
            _ => lexbuf.backtrack(),
        }
    }
    lexbuf.start();
    match __ferrelex_state_0(lexbuf) {
        0isize => {}
        _ => {}
    }
}
}
