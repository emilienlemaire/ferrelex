// Copyright (c) 2025-2026 Emilien Lemaire <emilien.lem@icloud.com>
// SPDX-License-Identifier: LGPL-3.0-only
// Licensed under the GNU Lesser General Public License v3.0, with the
// ferrelex Generated Code Exception. See the LICENSE file at the root
// of this repository for the full license text and exception terms.

//! # Ferrelex
//!
//! **Ferrelex** is a crate that enables you to write powerful, Unicode-aware lexers
//! entirely in Rust.
//!
//! Ferrelex is a lexer generator: it leverages procedural macros to let you describe
//! token patterns as readable regex expressions and match arms, then compiles them
//! into an efficient DFA at compile time — no runtime regex engine, no heap
//! allocation per token. You get the expressiveness of a traditional lexer generator
//! tool (think `ocamllex` or `flex`) with full access to Rust's type system, pattern
//! matching, and error handling.
//!
//! # Example
//!
//! ```rust
//! use ferrelex::{lexer::lex, lexbuf::{utf8::LexBuf, refiller::Utf8Refiller}};
//!
//! // You can use any type you'd like as the return type of your lexer.
//! #[derive(Debug, PartialEq, Eq)]
//! enum Token {
//!     Ascii(String),
//!     Lambda,
//!     Eof,
//!     Invalid
//! }
//!
//! // The lex! macro is where your regex definitions and match arms live.
//! lex! {
//!     // Regexes are constants of type `Regex`.
//!     const ASCII_LETTER: Regex = ('a'..='z') | ('A'..='Z');
//!     // Some characters don't fit in a Rust char literal — use a string instead.
//!     const LAMBDA: Regex = "λ";
//!
//!     // Your lexer lives inside an ordinary Rust function. It must accept a
//!     // `&mut LexBuf` argument; the return type and any extra arguments are
//!     // entirely up to you.
//!     pub fn my_lexer(lexbuf: &mut LexBuf) -> Token {
//!         // Place `#[lexer]` before your match expression to activate the DFA.
//!         #[lexer]
//!         match lexbuf {
//!             ASCII_LETTER => Token::Ascii(lexbuf.lexeme()),
//!             LAMBDA => Token::Lambda,
//!             eof => Token::Eof,
//!             // The last arm must be a wildcard to handle unmatched input.
//!             _ => Token::Invalid,
//!         }
//!     }
//! }
//!
//! fn main() {
//!     // Create a LexBuf from any input source — here, an owned String.
//!     let mut lexbuf = LexBuf::new(Utf8Refiller::new(String::from("λhello")));
//!
//!     // Call your lexer function once per token. The LexBuf advances automatically.
//!     assert_eq!(my_lexer(&mut lexbuf), Token::Lambda);
//!     assert_eq!(my_lexer(&mut lexbuf), Token::Ascii(String::from("h")));
//!     assert_eq!(my_lexer(&mut lexbuf), Token::Ascii(String::from("e")));
//!     assert_eq!(my_lexer(&mut lexbuf), Token::Ascii(String::from("l")));
//!     assert_eq!(my_lexer(&mut lexbuf), Token::Ascii(String::from("l")));
//!     assert_eq!(my_lexer(&mut lexbuf), Token::Ascii(String::from("o")));
//!
//!     // Once you reach the end of input, every call returns Token::Eof.
//!     assert_eq!(my_lexer(&mut lexbuf), Token::Eof);
//!     assert_eq!(my_lexer(&mut lexbuf), Token::Eof);
//! }
//! ```
//!
//! # Regex syntax
//!
//! Regex expressions inside `lex!` use Rust syntax and are evaluated at compile time:
//!
//! | Syntax | Meaning |
//! |--------|---------|
//! | `'a'` | Single character |
//! | `"hello"` | Literal string — sequence of its characters |
//! | `0x41` | Unicode code point as an integer literal |
//! | `'a'..='z'` | Inclusive character range |
//! | `'a'..'z'` | Exclusive character range (`a` to `y`) |
//! | `r1 \| r2` | Alternation — matches either `r1` or `r2` |
//! | `(r1, r2)` | Sequence — `r1` followed by `r2` |
//! | `Plus(r)` | One or more (`r+`) |
//! | `Star(r)` | Zero or more (`r*`) |
//! | `Opt(r)` | Zero or one (`r?`) |
//! | `Rep(r, n..=m)` | Between `n` and `m` repetitions (inclusive) |
//! | `Rep(r, n)` | Exactly `n` repetitions |
//! | `Compl(r)` | Complement — any character **not** in `r` ¹ |
//! | `Sub(r1, r2)` | Set difference — characters in `r1` but not in `r2` ¹ |
//! | `Intersect(r1, r2)` | Set intersection of two character classes ¹ |
//! | `AnyOf("abc")` | Any single character from the given string |
//! | `NAME` | Named regex defined with `const NAME: Regex = …` |
//!
//! ¹ `Compl`, `Sub`, and `Intersect` require both operands to be single-character-class
//! regexes. Prefer char literals (e.g. `'"'`) over single-character string literals
//! (e.g. `"\""`) as arguments — a string literal creates a sequence internally and
//! will be rejected by these operators.
//!
//! ## Built-in regex constants
//!
//! The following names are always in scope inside `lex!`, without any `const` definition:
//!
//! | Name | Matches |
//! |------|---------|
//! | `any` | Any single Unicode scalar value (does **not** match EOF) |
//! | `eof` | End of input |
//! | `digit_ascii` | `0`–`9` |
//! | `upper_ascii` | `A`–`Z` |
//! | `lower_ascii` | `a`–`z` |
//! | `alpha_ascii` | `A`–`Z` and `a`–`z` |
//! | `alnum_ascii` | `A`–`Z`, `a`–`z`, and `0`–`9` |
//! | `whitespace_ascii` | Space, tab (`\t`), newline (`\n`), carriage return (`\r`) |
//! | `word_ascii` | `alnum_ascii` plus `_` |
//!
//! All built-in shorthands are ASCII-only — the `_ascii` suffix makes that explicit.
//! For full Unicode coverage, use Unicode category and property names directly as
//! identifiers — see the sections below.
//!
//! ## Unicode General Categories
//!
//! The two-letter Unicode General Category codes are available as identifiers
//! directly inside `lex!`. Single-letter super-categories union all categories
//! sharing that letter prefix.
//!
//! ### Specific categories
//!
//! | Identifier | Long name | Description |
//! |------------|-----------|-------------|
//! | `Cc` | Control | C0/C1 control codes |
//! | `Cf` | Format | Invisible formatting indicators |
//! | `Cn` | Unassigned | Code points not yet assigned |
//! | `Co` | Private_Use | Private-use code points |
//! | `Cs` | Surrogate | Surrogate code points (U+D800–U+DFFF) |
//! | `Ll` | Lowercase_Letter | Lowercase letters (`a`–`z`, `à`, `α`, …) |
//! | `Lm` | Modifier_Letter | Modifier letters |
//! | `Lo` | Other_Letter | Other letters (ideographs, syllables, …) |
//! | `Lt` | Titlecase_Letter | Digraphic titlecase letters (e.g. `Dž`) |
//! | `Lu` | Uppercase_Letter | Uppercase letters (`A`–`Z`, `À`, `Α`, …) |
//! | `Mc` | Spacing_Mark | Spacing combining marks |
//! | `Me` | Enclosing_Mark | Enclosing combining marks |
//! | `Mn` | Nonspacing_Mark | Non-spacing combining marks |
//! | `Nd` | Decimal_Number | Decimal digits (`0`–`9`, `٠`–`٩`, …) |
//! | `Nl` | Letter_Number | Letter-like numerics (Roman numerals, …) |
//! | `No` | Other_Number | Other numerics (fractions, superscripts, …) |
//! | `Pc` | Connector_Punctuation | Connector punctuation (e.g. `_`) |
//! | `Pd` | Dash_Punctuation | Dashes and hyphens |
//! | `Pe` | Close_Punctuation | Closing brackets (`)`, `]`, `}`, …) |
//! | `Pf` | Final_Punctuation | Final quotation marks |
//! | `Pi` | Initial_Punctuation | Initial quotation marks |
//! | `Po` | Other_Punctuation | Other punctuation (`!`, `.`, `,`, …) |
//! | `Ps` | Open_Punctuation | Opening brackets (`(`, `[`, `{`, …) |
//! | `Sc` | Currency_Symbol | Currency symbols (`$`, `€`, `£`, …) |
//! | `Sk` | Modifier_Symbol | Non-spacing modifier symbols |
//! | `Sm` | Math_Symbol | Mathematical symbols (`+`, `<`, `=`, …) |
//! | `So` | Other_Symbol | Other symbols |
//! | `Zl` | Line_Separator | Line separator (U+2028) |
//! | `Zp` | Paragraph_Separator | Paragraph separator (U+2029) |
//! | `Zs` | Space_Separator | Space characters (U+0020, U+00A0, …) |
//!
//! ### Super-category aggregates
//!
//! | Identifier | Expands to |
//! |------------|------------|
//! | `C`  | `Cc` ∪ `Cf` ∪ `Cn` ∪ `Co` ∪ `Cs` |
//! | `L`  | `Ll` ∪ `Lm` ∪ `Lo` ∪ `Lt` ∪ `Lu` |
//! | `LC` | `Ll` ∪ `Lt` ∪ `Lu` (cased letters only) |
//! | `M`  | `Mc` ∪ `Me` ∪ `Mn` |
//! | `N`  | `Nd` ∪ `Nl` ∪ `No` |
//! | `P`  | `Pc` ∪ `Pd` ∪ `Pe` ∪ `Pf` ∪ `Pi` ∪ `Po` ∪ `Ps` |
//! | `S`  | `Sc` ∪ `Sk` ∪ `Sm` ∪ `So` |
//! | `Z`  | `Zl` ∪ `Zp` ∪ `Zs` |
//!
//! ## Unicode Derived Properties
//!
//! The following Unicode derived core property names are available as identifiers
//! inside `lex!` (Unicode 17.0.0, from `DerivedCoreProperties.txt`):
//!
//! | Identifier | Description |
//! |------------|-------------|
//! | `Alphabetic` | Letters and letter-like characters considered alphabetic |
//! | `Cased` | Characters with an uppercase, lowercase, or titlecase form |
//! | `Case_Ignorable` | Characters that do not affect casing of surrounding text |
//! | `Changes_When_Lowercased` | Characters whose lowercased form differs |
//! | `Changes_When_Uppercased` | Characters whose uppercased form differs |
//! | `Changes_When_Titlecased` | Characters whose titlecased form differs |
//! | `Changes_When_Casefolded` | Characters whose case-folded form differs |
//! | `Changes_When_Casemapped` | Union of the three `Changes_When_*cased` sets |
//! | `Default_Ignorable_Code_Point` | Code points that should be ignored by default |
//! | `Grapheme_Base` | Characters that can be the base of a grapheme cluster |
//! | `Grapheme_Extend` | Characters that extend a grapheme cluster |
//! | `Grapheme_Link` | Deprecated virama-based grapheme linking characters |
//! | `ID_Start` | Characters allowed at the start of an identifier |
//! | `ID_Continue` | Characters allowed inside an identifier (after `ID_Start`) |
//! | `XID_Start` | Stable version of `ID_Start` (closure under NFKC) |
//! | `XID_Continue` | Stable version of `ID_Continue` (closure under NFKC) |
//! | `Lowercase` | Characters with the `Lowercase` property |
//! | `Uppercase` | Characters with the `Uppercase` property |
//! | `Math` | Characters used in mathematical notation |
//!
//! # Skipping tokens with `#[skip]`
//!
//! Annotate a match arm with `#[skip]` to consume the matched input and restart the
//! DFA immediately, without returning to the caller. This is the idiomatic way to
//! discard whitespace or comments:
//!
//! ```rust,ignore
//! #[lexer]
//! match lexbuf {
//!     #[skip] whitespace_ascii       => {}
//!     #[skip] ("//", Star(any))      => {}  // line comment
//!     IDENT => Token::Ident(lexbuf.lexeme()),
//!     eof   => Token::Eof,
//!     _     => Token::Error,
//! }
//! ```
//!
//! # Specialized lexer functions
//!
//! Some tokens (string literals, block comments, heredocs, ...) require different
//! lexing rules mid-stream. The natural approach is to call a specialized lexer function
//! defined in the same `lex!` block. To avoid stack overflows on long inputs, use
//! `loop { #[lexer] match … }` inside the inner function instead of recursion:
//!
//! ```rust,ignore
//! lex! {
//!     const NOT_DQUOTE: Regex = Sub(any, '"');
//!
//!     pub fn token(lexbuf: &mut LexBuf) -> Token {
//!         #[lexer]
//!         match lexbuf {
//!             '"'   => lex_string(lexbuf),
//!             IDENT => Token::Ident(lexbuf.lexeme()),
//!             eof   => Token::Eof,
//!             _     => Token::Error,
//!         }
//!     }
//!
//!     // Inner lexer: iterative, no stack growth.
//!     pub fn lex_string(lexbuf: &mut LexBuf) -> Token {
//!         let mut acc = String::new();
//!         loop {
//!             #[lexer]
//!             match lexbuf {
//!                 '"'        => return Token::Str(acc),
//!                 NOT_DQUOTE => acc += lexbuf.lexeme_str(),
//!                 eof | _    => return Token::Error,
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! > **Note:** each `#[lexer]` match resets the token start position. If you need
//! > the position of the opening `"`, save it before calling the inner function:
//! > `let start = lexbuf.start_pos();`
//!
//! # `#[lexer]` options
//!
//! The `#[lexer]` attribute accepts optional settings:
//!
//! | Option | Effect |
//! |--------|--------|
//! | `no_line_tracking` | Disable line/column tracking for this match. Speeds up the DFA slightly; `start_pos().line` will always be `0`. |
//! | `allow_recursion` | Suppress the compile error for a direct recursive call to the enclosing function inside a match arm. |
//! | `case_insensitive` | Fold all character sets so patterns match regardless of case. `"select"` will match `SELECT`, `Select`, etc. Applies to every arm in the match. Note: iterates over every code point at compile time — fast for ASCII keyword sets, may slow compilation on large Unicode categories. |
//!
//! ```rust,ignore
//! #[lexer(no_line_tracking)]
//! match lexbuf {
//!     _ => ()
//! }
//! ```
//!
//! # Extracting the matched text
//!
//! Inside a match arm body, the `lexbuf` argument exposes the matched input:
//!
//! | Method | Returns |
//! |--------|---------|
//! | `lexbuf.lexeme()` | Owned `String` of the matched text |
//! | `lexbuf.lexeme_str()` | Borrowed `&str` (zero-copy) |
//! | `lexbuf.lexeme_bytes()` | Raw `&[u8]` — safe even on invalid UTF-8 |
//! | `lexbuf.lexeme_len()` | Number of Unicode scalar values matched |
//! | `lexbuf.lexeme_char(i)` | The `i`-th character (0-indexed), or `None` |
//! | `lexbuf.lexeme_chars()` | Iterator over the matched characters |
//!
//! # Position tracking
//!
//! Token positions are tracked automatically. These methods are accurate inside an
//! arm body, after a successful match:
//!
//! | Method | Returns |
//! |--------|---------|
//! | `lexbuf.start_pos()` | `Position` — first character of the current token |
//! | `lexbuf.end_pos()` | `Position` — character just past the token |
//! | `lexbuf.location()` | `Location` combining start and end |
//! | `lexbuf.set_filename(path)` | Attach a filename included in all subsequent positions |
//! | `lexbuf.set_line(n)` | Override the tracked line number (e.g. after a `#line N` directive) |
//!
//! `Position` contains `line` (1-indexed), `col` (0-indexed character offset from
//! the start of the line), and `filename`.
//!
//! # Invalid UTF-8
//!
//! When the input contains an invalid UTF-8 byte, the DFA fires the wildcard (`_`)
//! arm and sets `lexbuf.invalid_byte` to `Some(byte)`. Check this field to
//! distinguish invalid bytes from valid-but-unmatched characters:
//!
//! ```rust,ignore
//! _ => match lexbuf.invalid_byte {
//!     Some(b) => Token::InvalidByte(b),
//!     None    => Token::UnexpectedChar,
//! }
//! ```
//!
//! `lexbuf.invalid_byte` is cleared at the start of each new match. Calling
//! `lexbuf.lexeme()` or `lexbuf.lexeme_str()` in the wildcard arm when an invalid
//! byte triggered it produces a deprecation warning at compile time; use
//! `lexbuf.lexeme_bytes()` instead.
//!
//! # Input sources
//!
//! Four ready-to-use type aliases live in [`lexbuf::utf8`]:
//!
//! | Type alias | Input strategy | Char cache | Use when |
//! |------------|---------------|------------|----------|
//! | [`utf8::LexBuf`](lexbuf::utf8::LexBuf) | Buffered (streaming) | None | Default. Files, stdin, owned `String`. |
//! | [`utf8::SliceLexBuf`](lexbuf::utf8::SliceLexBuf) | Zero-copy slice | None | Entire input is already in memory as a `&str` or `&[u8]`. |
//! | [`utf8::CachingLexBuf`](lexbuf::utf8::CachingLexBuf) | Buffered (streaming) | `Vec<char>` | Streaming input with many multi-byte Unicode characters that are frequently backtracked over. |
//! | [`utf8::CachingSliceLexBuf`](lexbuf::utf8::CachingSliceLexBuf) | Zero-copy slice | `Vec<char>` | In-memory input with many multi-byte Unicode characters and frequent backtracking. |
//!
//! `utf8::LexBuf` and `utf8::CachingLexBuf` are backed by a
//! [`Refiller`](lexbuf::refiller::Refiller) that feeds chunks of bytes into an
//! internal buffer. Three `Refiller` implementations are provided:
//!
//! | Type | Use when |
//! |------|----------|
//! | [`Utf8Refiller`](lexbuf::refiller::Utf8Refiller) | You own a `String` |
//! | [`StrRefiller`](lexbuf::refiller::StrRefiller) | You have a `&str` (borrows the source) |
//! | [`ReadRefiller`](lexbuf::refiller::ReadRefiller) | You have any `std::io::Read` — files, stdin, sockets, … |
//!
//! Implement [`Refiller`](lexbuf::refiller::Refiller) yourself to support any other
//! streaming source. For complete control over the input and cache strategies,
//! use the underlying [`LexBuf<I, C>`](lexbuf::LexBuf) directly with your
//! own [`Input`](lexbuf::input::Input) and [`CharCache`](lexbuf::char_cache::CharCache)
//! implementations.

/// The [`lex!`](crate::lexer::lex) macro — the entry point for defining lexers.
///
/// Import it with:
///
/// ```rust,ignore
/// use ferrelex::lexer::lex;
/// ```
///
/// Everything you write inside `lex! { … }` is transformed at compile time into
/// ordinary Rust functions and static tables. The output is pure, dependency-free
/// Rust — no runtime library is linked for the lexing itself.
///
/// See the [crate-level documentation](crate) for the full syntax reference.
pub mod lexer {
    pub use ferrelex_macro::*;
}

/// The input buffer, input source, and cache strategy types.
///
/// Start with one of the ready-to-use aliases in [`utf8`](crate::lexbuf::utf8):
///
/// ```rust,ignore
/// // Streaming input (files, stdin, owned String) — the default:
/// use ferrelex::lexbuf::{utf8::LexBuf, refiller::Utf8Refiller};
///
/// // Entire input already in memory as a &str:
/// use ferrelex::lexbuf::utf8::SliceLexBuf;
/// ```
///
/// For advanced use, the full type-parameter form is [`LexBuf<I, C>`](crate::lexbuf::LexBuf)
/// where `I: Input` selects the byte source and `C: CharCache` selects the caching strategy.
/// See [`input::Input`](crate::lexbuf::input::Input) and
/// [`char_cache::CharCache`](crate::lexbuf::char_cache::CharCache) for details.
pub mod lexbuf {
    pub use ferrelex_core::char_cache;
    pub use ferrelex_core::input;
    pub use ferrelex_core::lexbuf::{utf8, CachingLexBuf, LexBuf};
    pub use ferrelex_core::refiller;
}

#[doc(hidden)]
pub mod __private {
    pub use ferrelex_core::lexbuf::WildcardLexBuf;
}
