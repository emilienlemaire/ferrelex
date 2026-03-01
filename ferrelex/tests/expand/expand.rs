// Copyright (c) 2025-2026 Emilien Lemaire <emilien.lem@icloud.com>
// SPDX-License-Identifier: LGPL-3.0-only
// Licensed under the GNU Lesser General Public License v3.0, with the
// ferrelex Generated Code Exception. See the LICENSE file at the root
// of this repository for the full license text and exception terms.

use ferrelex::{
    lexbuf::{utf8::LexBuf, refiller::Utf8Refiller},
    lexer::lex,
};

lex! {
    const ASCII_LETTERS: Regex = ('a'..'z') | ('A'..'Z');
    const LAMBDA: Regex = "λ";

    pub fn lex(lexbuf: &mut LexBuf) -> &'static str {
        #[lexer]
        match lexbuf {
            ASCII_LETTERS => "ascii",
            LAMBDA => "lambda",
            eof => "eof",
            _ => "error",
        }
    }
}

fn main() {
    let mut lb = LexBuf::new(Utf8Refiller::new(String::from("λa")));
    let _ = lex(&mut lb);
}
