// Copyright (c) 2025-2026 Emilien Lemaire <emilien.lem@icloud.com>
// SPDX-License-Identifier: LGPL-3.0-only
// Licensed under the GNU Lesser General Public License v3.0, with the
// ferrelex Generated Code Exception. See the LICENSE file at the root
// of this repository for the full license text and exception terms.

use super::Input;

/// Zero-copy input over an in-memory byte slice.
///
/// The entire input is available from construction; no heap allocation occurs and
/// [`refill`](Input::refill) is a no-op. This is the fastest input strategy for
/// lexing a `String` or `&str` already held in memory.
///
/// For streaming or file-backed sources use
/// [`BufferedInput`](super::BufferedInput) instead.
///
/// # Validity
///
/// The slice must contain valid UTF-8. Passing invalid UTF-8 will not cause
/// memory unsafety but may produce unexpected `-2` (invalid-byte) sentinels from
/// `next_int`.
#[derive(Debug, Clone)]
pub struct SliceInput<'a> {
    data: &'a [u8],
}

impl<'a> SliceInput<'a> {
    /// Wraps a raw byte slice. The slice must be valid UTF-8.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Wraps a `&str` as a byte slice. Always valid UTF-8.
    pub fn from_str(s: &'a str) -> Self {
        Self { data: s.as_bytes() }
    }
}

impl<'a> Input for SliceInput<'a> {
    /// Returns the full slice.
    #[inline(always)]
    fn as_slice(&self) -> &[u8] {
        self.data
    }

    #[inline(always)]
    fn len(&self) -> usize {
        self.data.len()
    }

    /// Always `true` — the entire input is present from the start.
    #[inline(always)]
    fn is_finished(&self) -> bool {
        true
    }

    /// No-op. The full slice is already available; nothing to fetch or compact.
    #[inline(always)]
    fn refill(&mut self, _keep_from: usize) -> usize {
        0
    }

    /// Always `0` — no bytes are ever removed from the front.
    #[inline(always)]
    fn bytes_removed(&self) -> usize {
        0
    }
}
