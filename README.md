# Ferrelex

**Ferrelex** is a compile-time lexer generator for Rust. Describe token patterns as
regex expressions and match arms inside the `lex!` macro; the macro compiles them into
an efficient DFA at build time — no runtime regex engine, no heap allocation per token.

Think `ocamllex` or `flex`, but entirely in Rust syntax.

## Features

- **Compile-time DFA** — all regex compilation happens in the proc macro; the emitted code is plain Rust with no runtime dependency on ferrelex
- **Unicode-aware** — full Unicode General Category and Derived Property support, multi-byte characters, case-folding
- **Familiar syntax** — patterns are Rust expressions; match arms are ordinary Rust code
- **`#[skip]` arms** — consume and discard tokens (whitespace, comments) without returning to the caller
- **Position tracking** — line, column, and filename attached to every token automatically
- **Invalid UTF-8 handling** — wildcard arm fires for invalid bytes; `lexbuf.invalid_byte` distinguishes them from valid-but-unmatched characters
- **Multiple input sources** — streaming (`Read`), borrowed slice (`&str`/`&[u8]`), with optional char caching for heavy Unicode workloads

## Installation

```toml
[dependencies]
ferrelex = "0.2"
```

## Quick start

```rust
use ferrelex::{lexer::lex, lexbuf::{utf8::LexBuf, refiller::Utf8Refiller}};

#[derive(Debug, PartialEq)]
enum Token { Ident(String), Number(String), Eof, Unknown }

lex! {
    const LETTER:  Regex = alpha_ascii | '_';
    const DIGIT:   Regex = digit_ascii;
    const IDENT:   Regex = LETTER | Plus((LETTER | DIGIT));
    const NUMBER:  Regex = Plus(DIGIT);

    pub fn next_token(lexbuf: &mut LexBuf) -> Token {
        #[lexer]
        match lexbuf {
            #[skip] whitespace_ascii        => {}
            IDENT                           => Token::Ident(lexbuf.lexeme()),
            NUMBER                          => Token::Number(lexbuf.lexeme()),
            eof                             => Token::Eof,
            _                               => Token::Unknown,
        }
    }
}

fn main() {
    let mut lb = LexBuf::new(Utf8Refiller::new("hello 42".to_string()));
    assert_eq!(next_token(&mut lb), Token::Ident("hello".into()));
    assert_eq!(next_token(&mut lb), Token::Number("42".into()));
    assert_eq!(next_token(&mut lb), Token::Eof);
}
```

## Regex syntax

Regex expressions inside `lex!` use Rust syntax and are evaluated at compile time:

| Syntax | Meaning |
|--------|---------|
| `'a'` | Single character |
| `"hello"` | Literal string — sequence of its characters |
| `0x41` | Unicode code point as an integer literal |
| `'a'..='z'` | Inclusive character range |
| `'a'..'z'` | Exclusive character range (`a` to `y`) |
| `r1 \| r2` | Alternation — matches either `r1` or `r2` |
| `(r1, r2)` | Sequence — `r1` followed by `r2` |
| `Plus(r)` | One or more (`r+`) |
| `Star(r)` | Zero or more (`r*`) |
| `Opt(r)` | Zero or one (`r?`) |
| `Rep(r, n..=m)` | Between `n` and `m` repetitions (inclusive) |
| `Rep(r, n)` | Exactly `n` repetitions |
| `Compl(r)` | Complement — any character **not** in `r` ¹ |
| `Sub(r1, r2)` | Set difference — characters in `r1` but not in `r2` ¹ |
| `Intersect(r1, r2)` | Set intersection ¹ |
| `AnyOf("abc")` | Any single character from the string |
| `NAME` | Named regex constant defined with `const NAME: Regex = …` |

¹ `Compl`, `Sub`, and `Intersect` require their operands to be single-character-class
regexes. Use char literals (`'"'`) rather than single-character strings (`"\""`) as
arguments — a string literal creates a sequence and will be rejected.

## Built-in constants

These names are always in scope inside `lex!`:

| Name | Matches |
|------|---------|
| `any` | Any Unicode scalar value (not EOF) |
| `eof` | End of input |
| `digit_ascii` | `0`–`9` |
| `upper_ascii` | `A`–`Z` |
| `lower_ascii` | `a`–`z` |
| `alpha_ascii` | `A`–`Z` and `a`–`z` |
| `alnum_ascii` | `A`–`Z`, `a`–`z`, `0`–`9` |
| `whitespace_ascii` | space, `\t`, `\n`, `\r` |
| `word_ascii` | `alnum_ascii` + `_` |

For full Unicode coverage, Unicode General Category codes (`Ll`, `Lu`, `Nd`, `L`, `N`, …)
and Derived Property names (`Alphabetic`, `XID_Start`, `XID_Continue`, …) are available
as identifiers directly inside `lex!`. See the [crate documentation](https://docs.rs/ferrelex)
for the complete list.

## `#[lexer]` options

```rust
#[lexer(no_line_tracking)]   // disable line/col tracking for a small speed gain
#[lexer(case_insensitive)]   // fold all patterns to match regardless of case
#[lexer(allow_recursion)]    // suppress the compile error for direct recursion
```

## Extracting matched text

Inside a match arm, `lexbuf` exposes the matched input:

| Method | Returns |
|--------|---------|
| `lexbuf.lexeme()` | `String` — owned copy of the matched text |
| `lexbuf.lexeme_str()` | `&str` — zero-copy borrow |
| `lexbuf.lexeme_bytes()` | `&[u8]` — raw bytes, safe on invalid UTF-8 |
| `lexbuf.lexeme_len()` | Number of matched Unicode scalar values |
| `lexbuf.lexeme_chars()` | Iterator over matched characters |

## Position tracking

| Method | Returns |
|--------|---------|
| `lexbuf.start_pos()` | `Position` — start of the current token |
| `lexbuf.end_pos()` | `Position` — just past the token |
| `lexbuf.location()` | `Location` — start + end combined |
| `lexbuf.set_filename(path)` | Attach a filename to all subsequent positions |
| `lexbuf.set_line(n)` | Override the tracked line number |

`Position` contains `line` (1-indexed), `col` (0-indexed), and `filename`.

## Input sources

| Type alias | Use when |
|------------|----------|
| `utf8::LexBuf` | Default — files, stdin, owned `String` |
| `utf8::SliceLexBuf` | Input already in memory as `&str` or `&[u8]` |
| `utf8::CachingLexBuf` | Streaming input with heavy Unicode backtracking |
| `utf8::CachingSliceLexBuf` | In-memory input with heavy Unicode backtracking |

Built-in `Refiller` implementations: `Utf8Refiller` (owned `String`), `StrRefiller`
(`&str`), `ReadRefiller` (any `std::io::Read`). Implement `Refiller` for custom sources.

## Workspace

| Crate | Description |
|-------|-------------|
| `ferrelex` | Public API — re-exports core and macro |
| `ferrelex_core` | Runtime types: `LexBuf`, `CSet`, Unicode data |
| `ferrelex_macro` | Proc macro: parses `lex!`, builds NFA/DFA, emits code |
| `ferrelex_gen` | Internal tool: regenerates Unicode data tables |

## AI assistance

Parts of this project were developed with the assistance of [Claude Code](https://claude.ai/code) (Anthropic). Claude contributed to portions of the implementation, most of the test suite, and the internal and crate-level documentation. All AI-generated content was reviewed, edited, and explicitly approved before being incorporated — no suggestion was accepted without deliberate human judgement.

## License

Licensed under the [GNU Lesser General Public License v3.0](LICENSE), with the
**ferrelex Generated Code Exception**: code emitted by the `lex!` macro is not
considered a derivative work of ferrelex and may be distributed under any terms.
