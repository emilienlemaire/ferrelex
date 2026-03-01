// Copyright (c) 2025-2026 Emilien Lemaire <emilien.lem@icloud.com>
// SPDX-License-Identifier: LGPL-3.0-only
// Licensed under the GNU Lesser General Public License v3.0, with the
// ferrelex Generated Code Exception. See the LICENSE file at the root
// of this repository for the full license text and exception terms.

use ferrelex::{lexbuf::utf8::LexBuf, lexer::lex};

lex! {
    const IDENT: Regex = Plus(('a'..='z') | ('A'..='Z'));

    pub fn lex_recursive(lexbuf: &mut LexBuf) -> String {
        #[lexer]
        match lexbuf {
            IDENT => lexbuf.lexeme() + &lex_recursive(lexbuf),
            eof => String::new(),
            _ => String::new(),
        }
    }
}

fn main() {}
