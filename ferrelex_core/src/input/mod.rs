// Copyright (c) 2025-2026 Emilien Lemaire <emilien.lem@icloud.com>
// SPDX-License-Identifier: LGPL-3.0-only
// Licensed under the GNU Lesser General Public License v3.0, with the
// ferrelex Generated Code Exception. See the LICENSE file at the root
// of this repository for the full license text and exception terms.

pub mod buffered;
pub mod slice;

pub use buffered::BufferedInput;
pub use slice::SliceInput;

/// Abstraction over the byte source for a [`LexBuf`](crate::lexbuf::LexBuf).
///
/// Two implementations ship out of the box:
///
/// - [`BufferedInput`] — for streaming or file-backed sources; maintains an
///   internal `Vec<u8>` window and refills it on demand from a
///   [`Refiller`](crate::refiller::Refiller). Right for stdin, files, sockets,
///   or any source where the full content is not already in memory.
///
/// - [`SliceInput`] — for in-memory sources; holds a `&[u8]` directly with no
///   heap allocation and no refill overhead. Right for lexing an owned `String`
///   or borrowed `&str` that is already loaded.
///
/// Implement this trait to support custom byte sources.
pub trait Input {
    /// View of the current byte window. Valid bytes occupy indices `0..self.len()`.
    fn as_slice(&self) -> &[u8];

    /// Number of valid bytes currently in the window.
    fn len(&self) -> usize;

    /// Whether the source is definitively exhausted — no future [`refill`](Input::refill)
    /// call will produce new bytes. Always `true` for [`SliceInput`].
    fn is_finished(&self) -> bool;

    /// Attempt to make more bytes available past `curr_bytes`.
    ///
    /// `keep_from` is the lowest byte index that must not be discarded (typically
    /// `start_bytes`, the beginning of the current lexeme). The implementation may
    /// compact the buffer by shifting out bytes before `keep_from` to make room.
    ///
    /// Returns the number of bytes removed from the front of the window; all stored
    /// byte positions must be decremented by this amount. Always returns `0` for
    /// [`SliceInput`] (no compaction ever occurs).
    fn refill(&mut self, keep_from: usize) -> usize;

    /// Cumulative count of bytes removed from the front of the window by previous
    /// [`refill`](Input::refill) calls. Used to compute absolute byte offsets.
    /// Always `0` for [`SliceInput`].
    fn bytes_removed(&self) -> usize;
}
