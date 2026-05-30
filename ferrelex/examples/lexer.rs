// Copyright (c) 2025-2026 Emilien Lemaire <emilien.lem@icloud.com>
// SPDX-License-Identifier: LGPL-3.0-only
// Licensed under the GNU Lesser General Public License v3.0, with the
// ferrelex Generated Code Exception. See the LICENSE file at the root
// of this repository for the full license text and exception terms.

extern crate ferrelex;

use ferrelex::{
    lexbuf::{utf8::LexBuf, refiller::Utf8Refiller},
    lexer,
};

#[derive(Debug)]
#[allow(dead_code)]
enum Token {
    Ascii(String),
    Lambda(String),
    Eof,
    Invalid,
}

lexer::lex! {
    const ASCII_LETTERS: Regex = ('a'..'z') | ('A'..'Z');
    const LAMBDA: Regex = "λ";
    fn lex(lexbuf: &mut LexBuf) -> Token {
        #[lexer]
        match lexbuf {
            ASCII_LETTERS => Token::Ascii(lexbuf.lexeme()) ,
            LAMBDA => Token::Lambda(lexbuf.lexeme()),
            eof => Token::Eof,
            _ => Token::Invalid,
        }
    }
}

fn main() {
    let mut lexbuf = LexBuf::new(Utf8Refiller::new(String::from("λhello")));
    dbg!(lex(&mut lexbuf));
    dbg!(lex(&mut lexbuf));
    dbg!(lex(&mut lexbuf));
    dbg!(lex(&mut lexbuf));
    dbg!(lex(&mut lexbuf));
    dbg!(lex(&mut lexbuf));
    dbg!(lex(&mut lexbuf));
    dbg!(lex(&mut lexbuf));
    dbg!(lex(&mut lexbuf));
    dbg!(lex(&mut lexbuf));
    dbg!(lex(&mut lexbuf));
    dbg!(lex(&mut lexbuf));
}
