// Copyright (c) 2025-2026 Emilien Lemaire <emilien.lem@icloud.com>
// SPDX-License-Identifier: LGPL-3.0-only
// Licensed under the GNU Lesser General Public License v3.0, with the
// ferrelex Generated Code Exception. See the LICENSE file at the root
// of this repository for the full license text and exception terms.

use ferrelex::{
    lexbuf::{utf8::LexBuf, refiller::Utf8Refiller},
    lexer::lex,
};

fn lb(s: &str) -> LexBuf {
    LexBuf::new(Utf8Refiller::new(s.to_string()))
}

// ── basic match + eof ────────────────────────────────────────────────────────

mod basic_lex {
    use super::*;

    lex! {
        const LETTER: Regex = ('a'..='z') | ('A'..='Z');

        pub fn lex(lexbuf: &mut LexBuf) -> &'static str {
            #[lexer]
            match lexbuf {
                LETTER => "letter",
                eof    => "eof",
                _      => "other",
            }
        }
    }

    #[test]
    fn basic_match_and_eof() {
        let mut b = lb("a");
        assert_eq!(lex(&mut b), "letter");
        assert_eq!(lex(&mut b), "eof");
        assert_eq!(lex(&mut b), "eof"); // idempotent
    }

    #[test]
    fn other_arm() {
        let mut b = lb("1");
        assert_eq!(lex(&mut b), "other");
    }
}

// ── #[skip] ─────────────────────────────────────────────────────────────────

mod skip_lex {
    use super::*;

    lex! {
        const WS:   Regex = ' ' | '\t' | '\n' | '\r';
        const WORD: Regex = Plus(('a'..='z') | ('A'..='Z'));

        pub fn lex(lexbuf: &mut LexBuf) -> Option<String> {
            #[lexer]
            match lexbuf {
                #[skip] WS => {}
                WORD       => Some(lexbuf.lexeme()),
                eof        => None,
                _          => Some("?".to_string()),
            }
        }
    }

    #[test]
    fn skip_whitespace() {
        let mut b = lb("  hello  ");
        assert_eq!(lex(&mut b).as_deref(), Some("hello"));
        assert_eq!(lex(&mut b), None);
    }

    #[test]
    fn skip_only_whitespace() {
        let mut b = lb("   ");
        assert_eq!(lex(&mut b), None);
    }
}

// ── first-match-wins (keyword before identifier) ─────────────────────────────

mod keyword_lex {
    use super::*;

    lex! {
        const WS:    Regex = ' ' | '\t' | '\n' | '\r';
        const IDENT: Regex = (('a'..='z') | ('A'..='Z'), Star(('a'..='z') | ('A'..='Z') | ('0'..='9')));

        pub fn lex(lexbuf: &mut LexBuf) -> &'static str {
            #[lexer]
            match lexbuf {
                "if"   => "kw_if",
                "else" => "kw_else",
                IDENT  => "ident",
                #[skip] WS => {}
                eof    => "eof",
                _      => "other",
            }
        }
    }

    #[test]
    fn first_match_wins_keyword() {
        let mut b = lb("if iffy else");
        assert_eq!(lex(&mut b), "kw_if");
        assert_eq!(lex(&mut b), "ident"); // "iffy" not a keyword
        assert_eq!(lex(&mut b), "kw_else");
        assert_eq!(lex(&mut b), "eof");
    }
}

// ── case_insensitive ─────────────────────────────────────────────────────────

mod case_insensitive_lex {
    use super::*;

    lex! {
        const WS: Regex = ' ' | '\t' | '\n' | '\r';

        pub fn lex(lexbuf: &mut LexBuf) -> &'static str {
            #[lexer(case_insensitive)]
            match lexbuf {
                "select"  => "kw_select",
                #[skip] WS => {}
                eof        => "eof",
                _          => "other",
            }
        }
    }

    #[test]
    fn lower() {
        let mut b = lb("select");
        assert_eq!(lex(&mut b), "kw_select");
    }

    #[test]
    fn upper() {
        let mut b = lb("SELECT");
        assert_eq!(lex(&mut b), "kw_select");
    }

    #[test]
    fn mixed() {
        let mut b = lb("Select sElEcT");
        assert_eq!(lex(&mut b), "kw_select");
        assert_eq!(lex(&mut b), "kw_select");
    }
}

// ── position tracking ────────────────────────────────────────────────────────

mod position_lex {
    use super::*;

    lex! {
        pub fn lex(lexbuf: &mut LexBuf) -> (usize, usize) {
            #[lexer]
            match lexbuf {
                ('a'..='z') | ('A'..='Z') | '\n' => {
                    let p = lexbuf.start_pos();
                    (p.line, p.col)
                }
                eof => {
                    let p = lexbuf.start_pos();
                    (p.line, p.col)
                }
                _ => {
                    let p = lexbuf.start_pos();
                    (p.line, p.col)
                }
            }
        }
    }

    #[test]
    fn line_increments_on_newline() {
        let mut b = lb("a\nb");
        let (l1, _) = lex(&mut b); // 'a'
        assert_eq!(l1, 1);
        let _ = lex(&mut b); // '\n'
        let (l3, _) = lex(&mut b); // 'b'
        assert_eq!(l3, 2);
    }

    #[test]
    fn col_increments_same_line() {
        let mut b = lb("ab");
        let (_, c1) = lex(&mut b); // 'a'
        assert_eq!(c1, 0);
        let (_, c2) = lex(&mut b); // 'b'
        assert_eq!(c2, 1);
    }
}

// ── no_line_tracking ─────────────────────────────────────────────────────────

mod no_line_tracking_lex {
    use super::*;

    lex! {
        pub fn lex(lexbuf: &mut LexBuf) -> usize {
            #[lexer(no_line_tracking)]
            match lexbuf {
                ('a'..='z') | '\n' => lexbuf.start_pos().line,
                eof => lexbuf.start_pos().line,
                _   => lexbuf.start_pos().line,
            }
        }
    }

    #[test]
    fn line_unchanged_across_newlines() {
        let mut b = lb("a\nb");
        let l1 = lex(&mut b); // 'a'
        let _ = lex(&mut b); // '\n'
        let l3 = lex(&mut b); // 'b'
        assert_eq!(l1, l3);
    }
}

// ── set_line ─────────────────────────────────────────────────────────────────

mod set_line_lex {
    use super::*;

    lex! {
        const WS: Regex = ' ' | '\t' | '\n' | '\r';

        pub fn lex(lexbuf: &mut LexBuf) -> usize {
            #[lexer]
            match lexbuf {
                ('a'..='z') | ('A'..='Z') => lexbuf.start_pos().line,
                #[skip] WS => {}
                eof => lexbuf.start_pos().line,
                _   => lexbuf.start_pos().line,
            }
        }
    }

    #[test]
    fn takes_effect_on_next_token() {
        let mut b = lb("a b");
        let _ = lex(&mut b); // consume 'a'
        b.set_line(99);
        let l = lex(&mut b); // 'b' — should report line 99
        assert_eq!(l, 99);
    }
}

// ── unicode categories ───────────────────────────────────────────────────────

mod unicode_cat_lex {
    use super::*;

    lex! {
        const WS: Regex = ' ' | '\t' | '\n' | '\r';

        pub fn lex(lexbuf: &mut LexBuf) -> &'static str {
            #[lexer]
            match lexbuf {
                Ll  => "lowercase",
                Lu  => "uppercase",
                Nd  => "digit",
                #[skip] WS => {}
                eof => "eof",
                _   => "other",
            }
        }
    }

    #[test]
    fn ll_matches_lowercase() {
        let mut b = lb("a");
        assert_eq!(lex(&mut b), "lowercase");
    }

    #[test]
    fn lu_matches_uppercase() {
        let mut b = lb("A");
        assert_eq!(lex(&mut b), "uppercase");
    }

    #[test]
    fn nd_matches_digit() {
        let mut b = lb("3");
        assert_eq!(lex(&mut b), "digit");
    }
}
