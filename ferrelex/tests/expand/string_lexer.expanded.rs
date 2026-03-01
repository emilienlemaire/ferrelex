use ferrelex::{
    lexbuf::{utf8::LexBuf, refiller::Utf8Refiller},
    lexer::lex,
};
enum Token {
    Ident(String),
    Str(String),
    Whitespace,
    Eof,
    Error,
}
#[automatically_derived]
impl ::core::fmt::Debug for Token {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self {
            Token::Ident(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(f, "Ident", &__self_0)
            }
            Token::Str(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(f, "Str", &__self_0)
            }
            Token::Whitespace => ::core::fmt::Formatter::write_str(f, "Whitespace"),
            Token::Eof => ::core::fmt::Formatter::write_str(f, "Eof"),
            Token::Error => ::core::fmt::Formatter::write_str(f, "Error"),
        }
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for Token {}
#[automatically_derived]
impl ::core::cmp::PartialEq for Token {
    #[inline]
    fn eq(&self, other: &Token) -> bool {
        let __self_discr = ::core::intrinsics::discriminant_value(self);
        let __arg1_discr = ::core::intrinsics::discriminant_value(other);
        __self_discr == __arg1_discr
            && match (self, other) {
                (Token::Ident(__self_0), Token::Ident(__arg1_0)) => __self_0 == __arg1_0,
                (Token::Str(__self_0), Token::Str(__arg1_0)) => __self_0 == __arg1_0,
                _ => true,
            }
    }
}
static __ferrelex_table_1: &[u8] = &[
    1u8, 1u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 1u8,
];
static __ferrelex_table_4: &[u8] = &[
    1u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 2u8, 2u8, 0u8, 0u8, 0u8, 0u8, 0u8,
    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 2u8,
    0u8, 3u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 4u8, 4u8,
    4u8, 4u8, 4u8, 4u8, 4u8, 4u8, 4u8, 4u8, 4u8, 4u8, 4u8, 4u8, 4u8, 4u8, 4u8, 4u8, 4u8,
    4u8, 4u8, 4u8, 4u8, 4u8, 4u8, 4u8, 0u8, 0u8, 0u8, 0u8, 4u8, 0u8, 4u8, 4u8, 4u8, 4u8,
    4u8, 4u8, 4u8, 4u8, 4u8, 4u8, 4u8, 4u8, 4u8, 4u8, 4u8, 4u8, 4u8, 4u8, 4u8, 4u8, 4u8,
    4u8, 4u8, 4u8, 4u8, 4u8,
];
static __ferrelex_table_3: &[u8] = &[
    1u8, 2u8, 2u8, 2u8, 2u8, 2u8, 2u8, 2u8, 2u8, 2u8, 2u8, 2u8, 2u8, 2u8, 2u8, 2u8, 2u8,
    2u8, 2u8, 2u8, 2u8, 2u8, 2u8, 2u8, 2u8, 2u8, 2u8, 2u8, 2u8, 2u8, 2u8, 2u8, 2u8, 2u8,
    2u8, 3u8,
];
static __ferrelex_table_2: &[u8] = &[
    1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8,
    1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 0u8, 0u8, 0u8, 0u8, 1u8, 0u8, 1u8, 1u8,
    1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8,
    1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8,
];
#[inline]
fn __ferrelex_partition_2(c: i32) -> i32 {
    if c <= 8i32 {
        -1i32
    } else {
        if c <= 32i32 {
            (__ferrelex_table_1[(c - 9i32) as usize] as i32 - 1)
        } else {
            -1i32
        }
    }
}
#[inline]
fn __ferrelex_partition_3(c: i32) -> i32 {
    if c <= 64i32 {
        -1i32
    } else {
        if c <= 122i32 {
            (__ferrelex_table_2[(c - 65i32) as usize] as i32 - 1)
        } else {
            -1i32
        }
    }
}
#[inline]
fn __ferrelex_partition_4(c: i32) -> i32 {
    if c <= 34i32 { (__ferrelex_table_3[(c - -1i32) as usize] as i32 - 1) } else { 1i32 }
}
#[inline]
fn __ferrelex_partition_1(c: i32) -> i32 {
    if c <= 122i32 {
        (__ferrelex_table_4[(c - -1i32) as usize] as i32 - 1)
    } else {
        -1i32
    }
}
/// Outer lexer: tokens outside of string literals.
pub fn token(lexbuf: &mut LexBuf) -> Token {
    lexbuf.start();
    let __ferrelex_result = {
        let mut state = 0i32;
        loop {
            match state {
                0i32 => {
                    let __ferrelex_c = lexbuf.next_int(true);
                    if __ferrelex_c < -1 {
                        break lexbuf.backtrack();
                    }
                    match __ferrelex_partition_1(__ferrelex_c) {
                        0i32 => {
                            break 3i32;
                        }
                        1i32 => {
                            state = 2i32;
                        }
                        2i32 => {
                            break 0i32;
                        }
                        3i32 => {
                            state = 4i32;
                        }
                        _ => break lexbuf.backtrack(),
                    }
                }
                2i32 => {
                    lexbuf.mark(2i32);
                    {
                        let __ferrelex_c = lexbuf.next_int(true);
                        if __ferrelex_c < -1 {
                            break lexbuf.backtrack();
                        }
                        match __ferrelex_partition_2(__ferrelex_c) {
                            0i32 => {
                                state = 2i32;
                            }
                            _ => break lexbuf.backtrack(),
                        }
                    }
                }
                4i32 => {
                    lexbuf.mark(1i32);
                    {
                        let __ferrelex_c = lexbuf.next_int(true);
                        if __ferrelex_c < -1 {
                            break lexbuf.backtrack();
                        }
                        match __ferrelex_partition_3(__ferrelex_c) {
                            0i32 => {
                                state = 4i32;
                            }
                            _ => break lexbuf.backtrack(),
                        }
                    }
                }
                _ => ::core::panicking::panic("internal error: entered unreachable code"),
            }
        }
    };
    match __ferrelex_result {
        0i32 => {
            let mut acc = String::new();
            lex_string(lexbuf, &mut acc).map(Token::Str).unwrap_or(Token::Error)
        }
        1i32 => Token::Ident(lexbuf.lexeme()),
        2i32 => Token::Whitespace,
        3i32 => Token::Eof,
        _ => {
            #[allow(unused_variables)]
            let lexbuf = ::ferrelex::__private::WildcardLexBuf(lexbuf);
            Token::Error
        }
    }
}
/// Inner lexer: accumulate string content after the opening `"` has been consumed.
pub fn lex_string(lexbuf: &mut LexBuf, acc: &mut String) -> Option<String> {
    loop {
        lexbuf.start();
        let __ferrelex_result = {
            let mut state = 0i32;
            loop {
                match state {
                    0i32 => {
                        let __ferrelex_c = lexbuf.next_int(true);
                        if __ferrelex_c < -1 {
                            break lexbuf.backtrack();
                        }
                        match __ferrelex_partition_4(__ferrelex_c) {
                            0i32 => {
                                break 2i32;
                            }
                            1i32 => {
                                break 1i32;
                            }
                            2i32 => {
                                break 0i32;
                            }
                            _ => break lexbuf.backtrack(),
                        }
                    }
                    _ => {
                        ::core::panicking::panic(
                            "internal error: entered unreachable code",
                        )
                    }
                }
            }
        };
        match __ferrelex_result {
            0i32 => return Some(acc.clone()),
            1i32 => *acc += lexbuf.lexeme_str(),
            2i32 => return None,
            _ => {
                #[allow(unused_variables)]
                let lexbuf = ::ferrelex::__private::WildcardLexBuf(lexbuf);
                return None;
            }
        }
    }
}
fn main() {
    let mut lb = LexBuf::new(Utf8Refiller::new(String::from("hello \"world\" foo")));
    match (&token(&mut lb), &Token::Ident("hello".into())) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&token(&mut lb), &Token::Whitespace) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&token(&mut lb), &Token::Str("world".into())) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&token(&mut lb), &Token::Whitespace) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&token(&mut lb), &Token::Ident("foo".into())) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&token(&mut lb), &Token::Eof) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
}
