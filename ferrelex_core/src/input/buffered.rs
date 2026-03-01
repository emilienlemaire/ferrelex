// Copyright (c) 2025-2026 Emilien Lemaire <emilien.lem@icloud.com>
// SPDX-License-Identifier: LGPL-3.0-only
// Licensed under the GNU Lesser General Public License v3.0, with the
// ferrelex Generated Code Exception. See the LICENSE file at the root
// of this repository for the full license text and exception terms.

use crate::refiller::Refiller;
use super::Input;

/// Streaming input backed by an internal `Vec<u8>` buffer.
///
/// Bytes are fetched from an underlying [`Refiller`] in chunks and stored in a
/// sliding window. When the window fills up the buffer is compacted (bytes before
/// the current lexeme start are discarded) and a fresh chunk is appended.
///
/// This is the right input strategy when the full source is not already in memory:
/// files, stdin, network sockets, or any [`std::io::Read`] source. For in-memory
/// strings prefer [`SliceInput`](super::SliceInput), which avoids the buffer
/// entirely.
#[derive(Debug)]
pub struct BufferedInput<R: Refiller> {
    buf: Vec<u8>,
    len: usize,
    chunk_size: usize,
    finished: bool,
    bytes_removed: usize,
    refiller: R,
}

impl<R: Refiller + Clone> Clone for BufferedInput<R> {
    fn clone(&self) -> Self {
        Self {
            buf: self.buf.clone(),
            len: self.len,
            chunk_size: self.chunk_size,
            finished: self.finished,
            bytes_removed: self.bytes_removed,
            refiller: self.refiller.clone(),
        }
    }
}

impl<R: Refiller> BufferedInput<R> {
    /// Creates a `BufferedInput` with the given refiller and chunk size.
    ///
    /// The buffer is pre-allocated to `chunk_size + 4` bytes (the extra 4 ensures
    /// a full 4-byte UTF-8 sequence can always be read without reallocation).
    pub fn new(refiller: R, chunk_size: usize) -> Self {
        let cap = chunk_size.saturating_add(4).max(4);
        Self {
            buf: vec![0u8; cap],
            len: 0,
            chunk_size,
            finished: false,
            bytes_removed: 0,
            refiller,
        }
    }
}

impl<R: Refiller> Input for BufferedInput<R> {
    #[inline]
    fn as_slice(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    #[inline]
    fn len(&self) -> usize {
        self.len
    }

    #[inline]
    fn is_finished(&self) -> bool {
        self.finished
    }

    fn refill(&mut self, keep_from: usize) -> usize {
        // Compact the buffer if we are running out of room.
        let shift = if self.len + self.chunk_size > self.buf.len() {
            let preserved = self.len.saturating_sub(keep_from);
            if preserved + self.chunk_size <= self.buf.len() {
                self.buf.copy_within(keep_from..keep_from + preserved, 0);
            } else {
                let new_cap = (self.buf.len() + self.chunk_size) * 2;
                self.buf.resize(new_cap, 0);
                self.buf.copy_within(keep_from..keep_from + preserved, 0);
            }
            self.len = preserved;
            self.bytes_removed += keep_from;
            keep_from
        } else {
            0
        };

        // Fetch the next chunk.
        if !self.finished {
            let n = self.refiller.refill(
                &mut self.buf[self.len..self.len + self.chunk_size],
                self.chunk_size,
            );
            if n == 0 {
                self.finished = true;
            } else {
                self.len += n;
            }
        }

        shift
    }

    #[inline]
    fn bytes_removed(&self) -> usize {
        self.bytes_removed
    }
}
