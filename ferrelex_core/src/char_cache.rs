// Copyright (c) 2025-2026 Emilien Lemaire <emilien.lem@icloud.com>
// SPDX-License-Identifier: LGPL-3.0-only
// Licensed under the GNU Lesser General Public License v3.0, with the
// ferrelex Generated Code Exception. See the LICENSE file at the root
// of this repository for the full license text and exception terms.

/// Strategy for storing decoded Unicode scalar values during a lexer scan.
///
/// Two implementations are provided:
///
/// - [`NoCache`] — the default on [`LexBuf`](crate::lexbuf::LexBuf). No heap
///   writes in the hot path; after backtrack the buffer bytes are re-decoded.
///   Best for ASCII-heavy inputs (source code, log files, English prose).
///
/// - [`WithCache`] — used by [`CachingLexBuf`](crate::lexbuf::CachingLexBuf).
///   Every decoded char is pushed to a `Vec`; after backtrack the Vec is
///   replayed instead of re-decoding. Best when patterns frequently match
///   multi-byte Unicode characters (CJK, emoji, non-Latin scripts) and
///   backtracking is common.
pub trait CharCache: Default + std::fmt::Debug + Clone {
    /// Store a newly decoded char (no-op for [`NoCache`]).
    fn push(&mut self, ch: char);
    /// Return the char at `idx`, or `None` when the cache is disabled.
    fn replay(&self, idx: usize) -> Option<char>;
    /// Number of chars currently stored (always 0 for [`NoCache`]).
    fn cache_len(&self) -> usize;
    /// Discard the first `n` chars (called by `start()` after a token is accepted).
    fn drain_front(&mut self, n: usize);
    /// Iterator over the chars of the current lexeme.
    ///
    /// `buf` is the raw byte slice `start_bytes..curr_bytes`; `cursor` is the
    /// number of chars decoded in the current token. Implementations may use
    /// either source depending on what they store.
    fn lexeme_chars_impl<'a>(
        &'a self,
        buf: &'a [u8],
        cursor: usize,
    ) -> impl Iterator<Item = char> + 'a;
    /// Single char at position `idx` in the current lexeme, or `None`.
    fn lexeme_char_impl(&self, buf: &[u8], idx: usize, cursor: usize) -> Option<char>;
}

// ── NoCache ───────────────────────────────────────────────────────────────────

/// Cache strategy that never stores decoded chars.
///
/// This is the default strategy used by [`LexBuf`](crate::lexbuf::LexBuf). The
/// forward scan path incurs no heap writes: decoded chars are returned
/// immediately without being saved.
///
/// After a [`backtrack`](crate::lexbuf::LexBuf::backtrack), the buffer bytes
/// are re-decoded from the rewound byte position. For ASCII (the common case in
/// source code) this costs a single byte comparison; for multi-byte chars it
/// costs the full 2–4 byte decode, but those bytes are still hot in L1/L2 cache
/// from the forward pass, so the cost is low in practice.
///
/// [`lexeme_chars`](crate::lexbuf::LexBuf::lexeme_chars) and
/// [`lexeme_char`](crate::lexbuf::LexBuf::lexeme_char) iterate the underlying
/// byte buffer on each call instead of reading a pre-decoded `Vec`.
///
/// **Prefer [`LexBuf`](crate::lexbuf::LexBuf)** (which uses `NoCache`) over
/// [`CachingLexBuf`](crate::lexbuf::CachingLexBuf) unless profiling shows a
/// clear win from the cache on your specific workload.
#[derive(Debug, Default, Clone)]
pub struct NoCache;

impl CharCache for NoCache {
    #[inline(always)]
    fn push(&mut self, _: char) {}

    #[inline(always)]
    fn replay(&self, _: usize) -> Option<char> {
        None
    }

    #[inline(always)]
    fn cache_len(&self) -> usize {
        0
    }

    #[inline(always)]
    fn drain_front(&mut self, _: usize) {}

    fn lexeme_chars_impl<'a>(
        &'a self,
        buf: &'a [u8],
        _cursor: usize,
    ) -> impl Iterator<Item = char> + 'a {
        // SAFETY: buf is always valid UTF-8 — only bytes produced by successful
        // decode_next_char_utf8 calls (start_bytes..curr_bytes) are passed here.
        unsafe { std::str::from_utf8_unchecked(buf) }.chars()
    }

    fn lexeme_char_impl(&self, buf: &[u8], idx: usize, _cursor: usize) -> Option<char> {
        unsafe { std::str::from_utf8_unchecked(buf) }
            .chars()
            .nth(idx)
    }
}

// ── WithCache ─────────────────────────────────────────────────────────────────

/// Cache strategy that stores every decoded char in a `Vec`.
///
/// This is the strategy used by [`CachingLexBuf`](crate::lexbuf::CachingLexBuf).
/// Each decoded char is pushed to the Vec on the forward pass. After a
/// [`backtrack`](crate::lexbuf::LexBuf::backtrack), chars in the current scan
/// window are replayed directly from the Vec instead of being re-decoded from
/// bytes, saving the 2–4 byte multi-byte decode per replayed char.
///
/// The tradeoff versus [`NoCache`]: every decoded char — including plain ASCII —
/// incurs a heap write on the forward pass. For ASCII-heavy inputs this write
/// cost dominates and [`LexBuf`](crate::lexbuf::LexBuf) (no cache) is faster.
///
/// **Prefer [`CachingLexBuf`](crate::lexbuf::CachingLexBuf)** when:
/// - your patterns match many multi-byte Unicode characters (CJK ideographs,
///   emoji, non-Latin scripts), **and**
/// - profiling confirms that `next_int` is still a bottleneck.
///
/// For all other workloads, start with [`LexBuf`](crate::lexbuf::LexBuf).
#[derive(Debug, Default, Clone)]
pub struct WithCache(Vec<char>);

impl CharCache for WithCache {
    #[inline]
    fn push(&mut self, ch: char) {
        self.0.push(ch);
    }

    #[inline]
    fn replay(&self, idx: usize) -> Option<char> {
        self.0.get(idx).copied()
    }

    #[inline]
    fn cache_len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    fn drain_front(&mut self, n: usize) {
        self.0.drain(..n);
    }

    fn lexeme_chars_impl<'a>(
        &'a self,
        _buf: &'a [u8],
        cursor: usize,
    ) -> impl Iterator<Item = char> + 'a {
        self.0[..cursor].iter().copied()
    }

    fn lexeme_char_impl(&self, _buf: &[u8], idx: usize, cursor: usize) -> Option<char> {
        if idx < cursor { self.0.get(idx).copied() } else { None }
    }
}
