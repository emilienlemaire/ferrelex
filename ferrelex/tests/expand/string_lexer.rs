// Copyright (c) 2025-2026 Emilien Lemaire <emilien.lem@icloud.com>
// SPDX-License-Identifier: LGPL-3.0-only
// Licensed under the GNU Lesser General Public License v3.0, with the
// ferrelex Generated Code Exception. See the LICENSE file at the root
// of this repository for the full license text and exception terms.

use ferrelex::{
    lexbuf::{utf8::LexBuf, refiller::Utf8Refiller},
    lexer::lex,
};

#[derive(Debug, PartialEq)]
enum Token {
    Ident(String),
    Str(String),
    Whitespace,
    Eof,
    Error,
}

lex! {
    const IDENT: Regex = Plus(('a'..='z') | ('A'..='Z') | '_');
    const WS: Regex = Plus(' ' | '\t' | '\n');
    const NOT_DQUOTE: Regex = Sub(any, '"');

    /// Outer lexer: tokens outside of string literals.
    pub fn token(lexbuf: &mut LexBuf) -> Token {
        #[lexer]
        match lexbuf {
            '"' => {
                let mut acc = String::new();
                lex_string(lexbuf, &mut acc).map(Token::Str).unwrap_or(Token::Error)
            }
            IDENT => Token::Ident(lexbuf.lexeme()),
            WS => Token::Whitespace,
            eof => Token::Eof,
            _ => Token::Error,
        }
    }

    /// Inner lexer: accumulate string content after the opening `"` has been consumed.
    pub fn lex_string(lexbuf: &mut LexBuf, acc: &mut String) -> Option<String> {
        loop {
            #[lexer]
            match lexbuf {
                '"' => return Some(acc.clone()),
                NOT_DQUOTE => *acc += lexbuf.lexeme_str(),
                eof => return None,
                _ => return None,
            }
        }
    }
}

fn main() {
    let mut lb = LexBuf::new(Utf8Refiller::new(String::from("hello \"world\" foo")));
    assert_eq!(token(&mut lb), Token::Ident("hello".into()));
    assert_eq!(token(&mut lb), Token::Whitespace);
    assert_eq!(token(&mut lb), Token::Str("world".into()));
    assert_eq!(token(&mut lb), Token::Whitespace);
    assert_eq!(token(&mut lb), Token::Ident("foo".into()));
    assert_eq!(token(&mut lb), Token::Eof);
}
