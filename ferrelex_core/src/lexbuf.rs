// Copyright (c) 2025-2026 Emilien Lemaire <emilien.lem@icloud.com>
// SPDX-License-Identifier: LGPL-3.0-only
// Licensed under the GNU Lesser General Public License v3.0, with the
// ferrelex Generated Code Exception. See the LICENSE file at the root
// of this repository for the full license text and exception terms.

use std::path::PathBuf;

use crate::{
    char_cache::{CharCache, NoCache, WithCache},
    input::{BufferedInput, Input, SliceInput},
    location::{Location, Position},
    refiller::Refiller,
};

#[derive(Debug)]
struct Utf8DecodeError {
    bad_byte: u8,
}

// ── LexBuf ───────────────────────────────────────────────────────────────────

/// Generic lexer input buffer, parameterised over input strategy and char cache.
///
/// Most users should use one of the ready-made type aliases rather than naming
/// this type directly:
///
/// | Alias | Input | Cache | When to use |
/// |-------|-------|-------|-------------|
/// | [`utf8::LexBuf`] | Buffered | None | **Default.** Streaming or file input, ASCII-heavy. |
/// | [`utf8::CachingLexBuf`] | Buffered | `Vec<char>` | Streaming input, multi-byte Unicode heavy. |
/// | [`utf8::SliceLexBuf`] | In-memory slice | None | In-memory `String`/`&str`, ASCII-heavy. |
/// | [`utf8::CachingSliceLexBuf`] | In-memory slice | `Vec<char>` | In-memory `String`/`&str`, multi-byte Unicode heavy. |
///
/// ## Choosing input strategy
///
/// Use [`BufferedInput`] (`utf8::LexBuf` / `utf8::CachingLexBuf`) when the
/// input arrives from a file, stdin, or any streaming source. The buffer
/// maintains a sliding window and refills automatically.
///
/// Use [`SliceInput`] (`utf8::SliceLexBuf` / `utf8::CachingSliceLexBuf`) when
/// the entire input is already in memory as a `String` or `&str`. There is no
/// heap allocation or refill overhead; the DFA reads directly from the original
/// bytes. There is no heap allocation or refill overhead; the DFA reads
/// directly from the original bytes.
///
/// ## Choosing cache strategy
///
/// Use [`NoCache`] (the default) when input is predominantly ASCII. Decoded
/// chars are not stored; `next_int` never touches the heap. After a backtrack
/// the bytes are re-decoded, which is cheap because they are still hot in cache.
///
/// Use [`WithCache`] (`Caching*` aliases) when your patterns match many
/// multi-byte Unicode characters (CJK, emoji, non-Latin scripts) **and**
/// profiling shows re-decoding on backtrack is measurably expensive.
///
/// # Accessing the matched text
///
/// - [`lexeme()`](LexBuf::lexeme) — owned `String`
/// - [`lexeme_str()`](LexBuf::lexeme_str) — borrowed `&str` (zero-copy)
/// - [`lexeme_bytes()`](LexBuf::lexeme_bytes) — raw `&[u8]`, always safe
/// - [`lexeme_chars()`](LexBuf::lexeme_chars) — iterator over `char`s
/// - [`lexeme_char(i)`](LexBuf::lexeme_char) — single character by index
/// - [`lexeme_len()`](LexBuf::lexeme_len) — number of Unicode scalar values
///
/// # Position tracking
///
/// - [`start_pos()`](LexBuf::start_pos) — position of the first character
/// - [`end_pos()`](LexBuf::end_pos) — position just past the last character
/// - [`location()`](LexBuf::location) — span combining both
/// - [`set_filename`](LexBuf::set_filename) — attach a filename to all positions
#[derive(Debug)]
pub struct LexBuf<I: Input, C: CharCache = NoCache> {
    input: I,
    // Scan position
    curr_bytes: usize,
    start_bytes: usize,
    marked_bytes: usize,
    marked_val: i32,
    marked_char_count: usize,
    marked_bytes_bol: usize,
    marked_line: usize,
    // Absolute char-based position tracking.
    // chars_before_cache + char_cursor = total chars decoded from file start.
    chars_before_cache: usize,
    chars_bol: usize, // absolute char index of the current line start
    marked_chars_bol: usize,
    start_line: usize,
    start_col: usize,
    // Char cache: ZST no-op for NoCache, Vec<char> for WithCache.
    char_cache: C,
    char_cursor: usize,
    filename: PathBuf,
    // Diagnostic fields — never touched by the generated state machine.
    line: usize,
    bytes_bol: usize, // window-relative byte index of the current line start
    /// Set to the raw byte value when the wildcard arm is triggered by an invalid
    /// UTF-8 byte; `None` when triggered by a valid char that matched no pattern.
    /// Cleared at each [`start()`](LexBuf::start).
    pub invalid_byte: Option<u8>,
}

// ── Constructors ─────────────────────────────────────────────────────────────

impl<I: Input, C: CharCache> LexBuf<I, C> {
    fn new_with_input(input: I) -> Self {
        Self {
            input,
            curr_bytes: 0,
            start_bytes: 0,
            marked_bytes: 0,
            marked_val: 0,
            marked_char_count: 0,
            marked_bytes_bol: 0,
            marked_line: 1,
            chars_before_cache: 0,
            chars_bol: 0,
            marked_chars_bol: 0,
            start_line: 0,
            start_col: 0,
            char_cache: C::default(),
            char_cursor: 0,
            filename: PathBuf::from(""),
            line: 1,
            bytes_bol: 0,
            invalid_byte: None,
        }
    }
}

/// Constructors for buffered (streaming) lexer buffers.
impl<R: Refiller, C: CharCache> LexBuf<BufferedInput<R>, C> {
    /// Creates a buffered `LexBuf` with a 512-byte refill chunk.
    ///
    /// Use [`with_chunk_size`](LexBuf::with_chunk_size) to tune the chunk size for
    /// high-throughput workloads.
    pub fn new(refiller: R) -> Self {
        Self::new_with_input(BufferedInput::new(refiller, 512))
    }

    /// Creates a buffered `LexBuf` with an explicit refill chunk size in bytes.
    ///
    /// Larger chunks reduce the number of [`Refiller`] calls at the cost of higher
    /// peak memory use. The default of 512 bytes suits most workloads.
    pub fn with_chunk_size(refiller: R, chunk_size: usize) -> Self {
        Self::new_with_input(BufferedInput::new(refiller, chunk_size))
    }
}

/// Constructors for zero-copy in-memory lexer buffers.
impl<'a, C: CharCache> LexBuf<SliceInput<'a>, C> {
    /// Creates a zero-copy `LexBuf` over a raw byte slice.
    ///
    /// The slice must be valid UTF-8. No heap allocation occurs; the DFA reads
    /// directly from `data`.
    pub fn from_slice(data: &'a [u8]) -> Self {
        Self::new_with_input(SliceInput::new(data))
    }

    /// Creates a zero-copy `LexBuf` from a string slice.
    ///
    /// No heap allocation occurs; the DFA reads directly from the `&str` bytes.
    pub fn from_str(s: &'a str) -> Self {
        Self::new_with_input(SliceInput::from_str(s))
    }
}

// ── Common methods ────────────────────────────────────────────────────────────

impl<I: Input, C: CharCache> LexBuf<I, C> {
    #[doc(hidden)]
    pub fn mark(&mut self, marked_val: i32) {
        self.marked_bytes = self.curr_bytes;
        self.marked_val = marked_val;
        self.marked_char_count = self.char_cursor;
        self.marked_bytes_bol = self.bytes_bol;
        self.marked_line = self.line;
        self.marked_chars_bol = self.chars_bol;
    }

    /// Marks the current position as the start of a new token.
    ///
    /// Called automatically by the generated DFA at the beginning of each
    /// `#[lexer] match`. You rarely need to call this directly.
    pub fn start(&mut self) {
        self.invalid_byte = None;
        self.start_bytes = self.curr_bytes;
        self.start_line = self.line;
        self.start_col =
            (self.chars_before_cache + self.char_cursor).saturating_sub(self.chars_bol);
        self.chars_before_cache += self.char_cursor;
        self.char_cache.drain_front(self.char_cursor);
        self.char_cursor = 0;
        self.mark(-1);
    }

    /// Attaches a source filename to all subsequent
    /// [`Position`] values.
    pub fn set_filename(&mut self, filename: impl Into<PathBuf>) {
        self.filename = filename.into();
    }

    /// Overrides the current line number.
    ///
    /// Use this after consuming a `#line N` or `#line N "file"` directive to keep
    /// position tracking in sync with the original source.
    pub fn set_line(&mut self, n: usize) {
        self.line = n;
        self.chars_bol = self.chars_before_cache + self.char_cursor;
        self.bytes_bol = self.curr_bytes;
        self.marked_line = n;
        self.marked_chars_bol = self.chars_bol;
        self.marked_bytes_bol = self.bytes_bol;
    }

    #[doc(hidden)]
    pub fn backtrack(&mut self) -> i32 {
        self.line = self.marked_line;
        self.bytes_bol = self.marked_bytes_bol;
        self.chars_bol = self.marked_chars_bol;
        self.curr_bytes = self.marked_bytes;
        self.char_cursor = self.marked_char_count;
        self.marked_val
    }

    #[doc(hidden)]
    pub fn new_line(&mut self) {
        self.line += 1;
        self.bytes_bol = self.curr_bytes;
        self.chars_bol = self.chars_before_cache + self.char_cursor;
    }

    /// Compact the input window and fetch more bytes.
    /// Adjusts all stored byte positions by the shift returned from the input.
    fn do_refill(&mut self) {
        let shift = self.input.refill(self.start_bytes);
        if shift > 0 {
            self.curr_bytes -= shift;
            self.marked_bytes = self.marked_bytes.saturating_sub(shift);
            self.bytes_bol = self.bytes_bol.saturating_sub(shift);
            self.start_bytes = 0;
        }
    }

    /// Decode the next UTF-8 scalar value from the input window at `curr_bytes`.
    ///
    /// Returns `None` when not enough bytes are available (caller should refill).
    /// Returns `Some(Err(_))` for invalid lead byte, bad continuation byte,
    /// over-long encoding, or surrogate (U+D800–U+DFFF).
    #[inline]
    fn decode_next_char_utf8(&mut self) -> Option<Result<char, Utf8DecodeError>> {
        let p = self.curr_bytes;
        let available = self.input.len();
        if p >= available {
            return None;
        }

        let b0 = self.input.as_slice()[p];

        if b0 < 0x80 {
            self.curr_bytes += 1;
            return Some(Ok(b0 as char));
        }

        let width = if (b0 & 0b1110_0000) == 0b1100_0000 {
            2
        } else if (b0 & 0b1111_0000) == 0b1110_0000 {
            3
        } else if (b0 & 0b1111_1000) == 0b1111_0000 {
            4
        } else {
            return Some(Err(Utf8DecodeError { bad_byte: b0 }));
        };

        if available - p < width {
            return None;
        }

        let slice = self.input.as_slice();
        let cp: u32 = match width {
            2 => {
                let b1 = slice[p + 1];
                if (b1 & 0b1100_0000) != 0b1000_0000 {
                    return Some(Err(Utf8DecodeError { bad_byte: b0 }));
                }
                let cp = ((b0 & 0b0001_1111) as u32) << 6 | ((b1 & 0b0011_1111) as u32);
                if cp < 0x80 {
                    return Some(Err(Utf8DecodeError { bad_byte: b0 }));
                }
                cp
            }
            3 => {
                let b1 = slice[p + 1];
                let b2 = slice[p + 2];
                if (b1 & 0b1100_0000) != 0b1000_0000 || (b2 & 0b1100_0000) != 0b1000_0000 {
                    return Some(Err(Utf8DecodeError { bad_byte: b0 }));
                }
                let cp = ((b0 & 0b0000_1111) as u32) << 12
                    | ((b1 & 0b0011_1111) as u32) << 6
                    | ((b2 & 0b0011_1111) as u32);
                if cp < 0x800 {
                    return Some(Err(Utf8DecodeError { bad_byte: b0 }));
                }
                cp
            }
            _ => {
                let b1 = slice[p + 1];
                let b2 = slice[p + 2];
                let b3 = slice[p + 3];
                if (b1 & 0b1100_0000) != 0b1000_0000
                    || (b2 & 0b1100_0000) != 0b1000_0000
                    || (b3 & 0b1100_0000) != 0b1000_0000
                {
                    return Some(Err(Utf8DecodeError { bad_byte: b0 }));
                }
                let cp = ((b0 & 0b0000_0111) as u32) << 18
                    | ((b1 & 0b0011_1111) as u32) << 12
                    | ((b2 & 0b0011_1111) as u32) << 6
                    | ((b3 & 0b0011_1111) as u32);
                if cp < 0x10000 {
                    return Some(Err(Utf8DecodeError { bad_byte: b0 }));
                }
                cp
            }
        };

        if cp > 0x10FFFF || (0xD800..=0xDFFF).contains(&cp) {
            return Some(Err(Utf8DecodeError { bad_byte: b0 }));
        }

        let ch = unsafe { char::from_u32_unchecked(cp) };
        self.curr_bytes += width;
        Some(Ok(ch))
    }

    /// Advance by one Unicode scalar value and return it as `i32`.
    ///
    /// Sentinels: `>= 0` = code point; `-1` = EOF; `-2` = invalid UTF-8 byte
    /// (also sets [`invalid_byte`](LexBuf::invalid_byte)).
    ///
    /// Pass `track_lines = true` (the default) to update line/column on `\n`.
    #[inline]
    pub fn next_int(&mut self, track_lines: bool) -> i32 {
        // Replay already-decoded chars after a backtrack (WithCache only;
        // cache_len() is always 0 for NoCache so this branch is never taken).
        if self.char_cursor < self.char_cache.cache_len() {
            let ch = self.char_cache.replay(self.char_cursor).unwrap();
            self.char_cursor += 1;
            self.curr_bytes += ch.len_utf8();
            if track_lines && ch == '\n' {
                self.new_line();
            }
            return ch as i32;
        }

        // ASCII fast path: one bounds check, one byte load, no heap writes.
        if self.curr_bytes < self.input.len() {
            let b0 = self.input.as_slice()[self.curr_bytes];
            if b0 < 0x80 {
                self.curr_bytes += 1;
                self.char_cache.push(b0 as char); // no-op for NoCache
                self.char_cursor += 1;
                if track_lines && b0 == b'\n' {
                    self.new_line();
                }
                return b0 as i32;
            }
        }

        self.next_int_slow(track_lines)
    }

    /// Slow path: multi-byte UTF-8, refill, or error.
    ///
    /// Marked `#[cold]` + `#[inline(never)]` so the fast path above stays tight.
    #[cold]
    #[inline(never)]
    fn next_int_slow(&mut self, track_lines: bool) -> i32 {
        loop {
            if !self.input.is_finished() && self.curr_bytes == self.input.len() {
                self.do_refill();
            }
            if self.input.is_finished() && self.curr_bytes == self.input.len() {
                break -1;
            } else {
                match self.decode_next_char_utf8() {
                    None => {
                        if self.input.is_finished() {
                            break -1;
                        }
                        self.do_refill();
                    }
                    Some(Ok(ch)) => {
                        if track_lines && ch == '\n' {
                            self.new_line();
                        }
                        self.char_cache.push(ch);
                        self.char_cursor += 1;
                        break ch as i32;
                    }
                    Some(Err(e)) => {
                        self.invalid_byte = Some(e.bad_byte);
                        self.curr_bytes += 1;
                        // Advance start/marked past the bad byte only when no valid
                        // match has been recorded yet, so backtrack() makes forward
                        // progress and lexeme_str() stays valid UTF-8.
                        if self.marked_bytes == self.start_bytes {
                            self.start_bytes = self.curr_bytes;
                            self.marked_bytes = self.curr_bytes;
                            self.marked_char_count = self.char_cursor;
                        }
                        break -2;
                    }
                }
            }
        }
    }

    /// Returns the next character, or `None` at EOF or on an invalid UTF-8 byte.
    pub fn next_char(&mut self) -> Option<char> {
        match self.next_int(true) {
            c if c >= 0 => char::from_u32(c as u32),
            _ => None,
        }
    }

    // ── Lexeme accessors ──────────────────────────────────────────────────────

    /// Returns the matched text as an owned `String`.
    pub fn lexeme(&self) -> String {
        unsafe {
            String::from_utf8_unchecked(
                self.input.as_slice()[self.start_bytes..self.curr_bytes].to_vec(),
            )
        }
    }

    /// Returns the matched text as a borrowed `&str` (zero-copy).
    pub fn lexeme_str(&self) -> &str {
        unsafe {
            std::str::from_utf8_unchecked(
                &self.input.as_slice()[self.start_bytes..self.curr_bytes],
            )
        }
    }

    /// Returns the raw bytes of the current lexeme.
    ///
    /// Safe to call even in the wildcard arm when
    /// [`invalid_byte`](LexBuf::invalid_byte) is set.
    pub fn lexeme_bytes(&self) -> &[u8] {
        &self.input.as_slice()[self.start_bytes..self.curr_bytes]
    }

    /// Returns the char at position `idx` in the current lexeme (0-indexed).
    ///
    /// With [`NoCache`] this iterates the byte slice — O(n).
    /// With [`WithCache`] this is O(1).
    pub fn lexeme_char(&self, idx: usize) -> Option<char> {
        self.char_cache.lexeme_char_impl(
            &self.input.as_slice()[self.start_bytes..self.curr_bytes],
            idx,
            self.char_cursor,
        )
    }

    /// Returns the number of Unicode scalar values in the current lexeme.
    pub fn lexeme_len(&self) -> usize {
        self.char_cursor
    }

    /// Returns an iterator over the chars of the current lexeme.
    ///
    /// With [`NoCache`] this decodes the byte slice on each call.
    /// With [`WithCache`] this iterates the pre-decoded `Vec`.
    pub fn lexeme_chars(&self) -> impl Iterator<Item = char> + '_ {
        self.char_cache.lexeme_chars_impl(
            &self.input.as_slice()[self.start_bytes..self.curr_bytes],
            self.char_cursor,
        )
    }

    // ── Position / EOF ────────────────────────────────────────────────────────

    /// Returns `true` once the input source is exhausted and all bytes consumed.
    pub fn is_eof(&self) -> bool {
        self.input.is_finished() && self.curr_bytes >= self.input.len()
    }

    /// Absolute byte offset of the current position from the start of the input.
    pub fn byte_offset(&self) -> usize {
        self.input.bytes_removed() + self.curr_bytes
    }

    /// Position of the first character of the current lexeme.
    pub fn start_pos(&self) -> Position {
        Position {
            line: self.start_line,
            col: self.start_col,
            filename: self.filename.clone(),
        }
    }

    /// Position just past the last character of the current lexeme.
    pub fn end_pos(&self) -> Position {
        Position {
            line: self.line,
            col: (self.chars_before_cache + self.char_cursor).saturating_sub(self.chars_bol),
            filename: self.filename.clone(),
        }
    }

    /// Span covering the entire current lexeme.
    pub fn location(&self) -> Location {
        Location {
            start: self.start_pos(),
            end: self.end_pos(),
        }
    }
}

// ── Clone ─────────────────────────────────────────────────────────────────────

impl<I: Input + Clone, C: CharCache> Clone for LexBuf<I, C> {
    fn clone(&self) -> Self {
        Self {
            input: self.input.clone(),
            curr_bytes: self.curr_bytes,
            start_bytes: self.start_bytes,
            marked_bytes: self.marked_bytes,
            marked_val: self.marked_val,
            marked_char_count: self.marked_char_count,
            marked_bytes_bol: self.marked_bytes_bol,
            marked_line: self.marked_line,
            chars_before_cache: self.chars_before_cache,
            chars_bol: self.chars_bol,
            marked_chars_bol: self.marked_chars_bol,
            start_line: self.start_line,
            start_col: self.start_col,
            char_cache: self.char_cache.clone(),
            char_cursor: self.char_cursor,
            filename: self.filename.clone(),
            line: self.line,
            bytes_bol: self.bytes_bol,
            invalid_byte: self.invalid_byte,
        }
    }
}

// ── WildcardLexBuf ────────────────────────────────────────────────────────────

/// Wrapper around `&mut LexBuf` used in the generated wildcard arm.
///
/// Exposes all `LexBuf` methods via `Deref`/`DerefMut` but marks the `lexeme*`
/// methods as deprecated to warn that they may return empty or invalid data when
/// the wildcard arm fires due to an invalid UTF-8 byte.
pub struct WildcardLexBuf<'a, I: Input, C: CharCache = NoCache>(pub &'a mut LexBuf<I, C>);

impl<'a, I: Input, C: CharCache> WildcardLexBuf<'a, I, C> {
    #[deprecated = "lexeme() may return \"\" in the wildcard arm when fired by an invalid UTF-8 byte; check `invalid_byte` or use the `any` regex instead"]
    pub fn lexeme(&self) -> String {
        self.0.lexeme()
    }

    #[deprecated = "lexeme_str() may return \"\" in the wildcard arm when fired by an invalid UTF-8 byte; check `invalid_byte` or use the `any` regex instead"]
    pub fn lexeme_str(&self) -> &str {
        self.0.lexeme_str()
    }

    #[deprecated = "lexeme_char() may return None in the wildcard arm when fired by an invalid UTF-8 byte; check `invalid_byte` or use the `any` regex instead"]
    pub fn lexeme_char(&self, idx: usize) -> Option<char> {
        self.0.lexeme_char(idx)
    }

    #[deprecated = "lexeme_len() may return 0 in the wildcard arm when fired by an invalid UTF-8 byte; check `invalid_byte` or use the `any` regex instead"]
    pub fn lexeme_len(&self) -> usize {
        self.0.lexeme_len()
    }
}

impl<'a, I: Input, C: CharCache> std::ops::Deref for WildcardLexBuf<'a, I, C> {
    type Target = LexBuf<I, C>;
    fn deref(&self) -> &LexBuf<I, C> {
        self.0
    }
}

impl<'a, I: Input, C: CharCache> std::ops::DerefMut for WildcardLexBuf<'a, I, C> {
    fn deref_mut(&mut self) -> &mut LexBuf<I, C> {
        self.0
    }
}

// ── Type aliases ──────────────────────────────────────────────────────────────

/// A [`LexBuf`] that caches decoded chars for fast backtrack replay.
///
/// Use when patterns frequently match multi-byte Unicode characters. See the
/// [`LexBuf`] docs for the full strategy comparison.
pub type CachingLexBuf<I> = LexBuf<I, WithCache>;

// ── utf8 convenience module ───────────────────────────────────────────────────

/// Ready-to-use type aliases for UTF-8 input.
///
/// Import the one that matches your workload and use it directly — no type
/// parameters required.
///
/// ```rust,ignore
/// // Most lexers: streaming input, ASCII-heavy source code.
/// use ferrelex::lexbuf::utf8::LexBuf;
///
/// // In-memory string, ASCII-heavy:
/// use ferrelex::lexbuf::utf8::SliceLexBuf;
///
/// // Multi-byte Unicode heavy (CJK, emoji, …):
/// use ferrelex::lexbuf::utf8::{CachingLexBuf, CachingSliceLexBuf};
/// ```
pub mod utf8 {
    use crate::char_cache::WithCache;
    use crate::input::{BufferedInput, SliceInput};
    use crate::refiller::{StrRefiller, Utf8Refiller};

    /// **Default lexer buffer.** Buffered UTF-8 input, no char cache.
    ///
    /// Start here. Works with any [`Refiller`](crate::refiller::Refiller) and handles
    /// streaming sources. Switch to [`SliceLexBuf`] if the entire input is already
    /// in memory and you want to eliminate buffer overhead.
    pub type LexBuf = super::LexBuf<BufferedInput<Utf8Refiller>>;

    /// Buffered UTF-8 input with char caching.
    ///
    /// Use when input is streamed and patterns match many multi-byte Unicode
    /// characters. Trades a heap write per char for faster backtrack replay.
    pub type CachingLexBuf = super::LexBuf<BufferedInput<Utf8Refiller>, WithCache>;

    /// Zero-copy UTF-8 input from an in-memory string or slice, no char cache.
    ///
    /// Use when the entire input is already in a `String` or `&str`. No buffer
    /// allocation occurs; the DFA reads directly from the original bytes. This
    /// eliminates the buffering overhead present in [`LexBuf`] and provides
    /// performance closest to dedicated in-memory lexers.
    pub type SliceLexBuf<'a> = super::LexBuf<SliceInput<'a>>;

    /// Zero-copy UTF-8 input with char caching.
    ///
    /// Combines in-memory slice access with char caching. Use when the input is
    /// in-memory and patterns match many multi-byte Unicode characters.
    pub type CachingSliceLexBuf<'a> = super::LexBuf<SliceInput<'a>, WithCache>;

    /// Buffered UTF-8 input backed by a borrowed `&str`, no char cache.
    ///
    /// Like [`LexBuf`] but uses a [`StrRefiller`](crate::refiller::StrRefiller)
    /// that borrows the source instead of owning it — no allocation occurs at
    /// construction time. Useful when you have a `&str` and want streaming
    /// buffer semantics without copying the data.
    pub type StrLexBuf<'a> = super::LexBuf<BufferedInput<StrRefiller<'a>>>;
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::BufferedInput;
    use crate::refiller::{ReadRefiller, Utf8Refiller};

    fn lb(s: &str) -> utf8::LexBuf {
        utf8::LexBuf::new(Utf8Refiller::new(s.to_string()))
    }

    fn slb(s: &str) -> utf8::SliceLexBuf<'_> {
        utf8::SliceLexBuf::from_str(s)
    }

    fn clb(s: &str) -> utf8::CachingLexBuf {
        utf8::CachingLexBuf::new(Utf8Refiller::new(s.to_string()))
    }

    #[test]
    fn is_eof_on_empty_input() {
        let mut b = lb("");
        b.start();
        assert_eq!(b.next_int(false), -1);
        assert!(b.is_eof());
    }

    #[test]
    fn is_eof_not_set_while_input_remains() {
        let mut b = lb("a");
        b.start();
        assert_eq!(b.next_int(false), 'a' as i32);
        assert!(!b.is_eof());
    }

    #[test]
    fn slice_lexbuf_basic() {
        let mut b = slb("hello");
        b.start();
        assert_eq!(b.next_int(false), 'h' as i32);
        assert_eq!(b.next_int(false), 'e' as i32);
        assert_eq!(b.next_int(false), 'l' as i32);
        assert_eq!(b.next_int(false), 'l' as i32);
        assert_eq!(b.next_int(false), 'o' as i32);
        assert_eq!(b.next_int(false), -1);
        assert!(b.is_eof());
    }

    #[test]
    fn slice_lexbuf_multibyte() {
        let mut b = slb("αβ");
        b.start();
        assert_eq!(b.next_int(false), 'α' as i32);
        assert_eq!(b.next_int(false), 'β' as i32);
        assert_eq!(b.next_int(false), -1);
    }

    #[test]
    fn set_line_changes_reported_line() {
        let mut b = lb("ab");
        b.start();
        b.next_int(true);
        b.mark('a' as i32);
        assert_eq!(b.start_pos().line, 1);
        b.set_line(42);
        b.start();
        b.next_int(true);
        b.mark('b' as i32);
        assert_eq!(b.start_pos().line, 42);
    }

    #[test]
    fn set_line_not_undone_by_backtrack() {
        let mut b = lb("ab");
        b.start();
        b.next_int(false);
        b.mark(0);
        b.set_line(99);
        b.backtrack();
        b.start();
        assert_eq!(b.start_pos().line, 99);
    }

    #[test]
    fn invalid_utf8_returns_sentinel_and_sets_flag() {
        let data = vec![0xFFu8];
        let mut b: LexBuf<BufferedInput<ReadRefiller<_>>> =
            LexBuf::new(ReadRefiller::new(std::io::Cursor::new(data)));
        b.start();
        assert_eq!(b.next_int(false), -2);
        assert_eq!(b.invalid_byte, Some(0xFF));
    }

    #[test]
    fn invalid_byte_cleared_on_start() {
        let data = vec![0xFFu8, b'a'];
        let mut b: LexBuf<BufferedInput<ReadRefiller<_>>> =
            LexBuf::new(ReadRefiller::new(std::io::Cursor::new(data)));
        b.start();
        b.next_int(false);
        assert_eq!(b.invalid_byte, Some(0xFF));
        b.start();
        assert_eq!(b.invalid_byte, None);
    }

    #[test]
    fn caching_lexbuf_replays_multibyte_after_backtrack() {
        let mut b = clb("αβ");
        b.start();
        let alpha = b.next_int(false);
        b.mark(0);
        let beta = b.next_int(false);
        b.backtrack();
        let beta2 = b.next_int(false);
        assert_eq!(alpha, 'α' as i32);
        assert_eq!(beta, 'β' as i32);
        assert_eq!(beta2, 'β' as i32);
    }

    #[test]
    fn nocache_lexbuf_redecodes_multibyte_after_backtrack() {
        let mut b = lb("αβ");
        b.start();
        let alpha = b.next_int(false);
        b.mark(0);
        let beta = b.next_int(false);
        b.backtrack();
        let beta2 = b.next_int(false);
        assert_eq!(alpha, 'α' as i32);
        assert_eq!(beta, 'β' as i32);
        assert_eq!(beta2, 'β' as i32);
    }

    #[test]
    fn slice_lexbuf_redecodes_multibyte_after_backtrack() {
        let mut b = slb("αβ");
        b.start();
        let alpha = b.next_int(false);
        b.mark(0);
        let beta = b.next_int(false);
        b.backtrack();
        let beta2 = b.next_int(false);
        assert_eq!(alpha, 'α' as i32);
        assert_eq!(beta, 'β' as i32);
        assert_eq!(beta2, 'β' as i32);
    }
}
