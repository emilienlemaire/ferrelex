// Copyright (c) 2025-2026 Emilien Lemaire <emilien.lem@icloud.com>
// SPDX-License-Identifier: LGPL-3.0-only
// Licensed under the GNU Lesser General Public License v3.0, with the
// ferrelex Generated Code Exception. See the LICENSE file at the root
// of this repository for the full license text and exception terms.

/// Provides raw bytes to a [`LexBuf`](crate::lexbuf::LexBuf) on demand.
///
/// Implement this trait to connect a `LexBuf` to any input source. The three
/// built-in implementations cover the most common cases:
///
/// - [`Utf8Refiller`] — owned `String`
/// - [`StrRefiller`] — borrowed `&str` (zero-copy)
/// - [`ReadRefiller`] — any [`std::io::Read`] (files, stdin, sockets, …)
pub trait Refiller {
    /// Fill `buf[..len]` with the next bytes from the input source and return
    /// the number of bytes written. Returning `0` signals end of input.
    fn refill(&mut self, buf: &mut [u8], len: usize) -> usize;
}

/// A [`Refiller`] backed by an owned `String`.
///
/// The entire string is consumed linearly. Use [`StrRefiller`] if you already
/// have a `&str` and want to avoid the allocation.
#[derive(Debug, Clone)]
pub struct Utf8Refiller {
    input: String,
    curr_byte: usize,
}

impl Utf8Refiller {
    /// Creates a new `Utf8Refiller` from an owned `String`.
    pub fn new(input: String) -> Self {
        Self {
            input,
            curr_byte: 0,
        }
    }
}

impl Refiller for Utf8Refiller {
    fn refill(&mut self, buf: &mut [u8], len: usize) -> usize {
        let bytes = &self.input.as_bytes()[self.curr_byte..];
        let read_len = if bytes.len() > len { len } else { bytes.len() };
        buf[..read_len].copy_from_slice(&bytes[..read_len]);
        self.curr_byte = self.curr_byte.saturating_add(read_len);
        read_len
    }
}

/// A [`Refiller`] backed by a borrowed `&str` (zero-copy).
///
/// The `LexBuf` borrows from the string for its lifetime; no allocation occurs.
/// Use [`Utf8Refiller`] when you need to own the input.
#[derive(Debug, Clone)]
pub struct StrRefiller<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> StrRefiller<'a> {
    /// Creates a new `StrRefiller` that borrows the given string slice.
    pub fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            pos: 0,
        }
    }
}

impl<'a> Refiller for StrRefiller<'a> {
    fn refill(&mut self, buf: &mut [u8], len: usize) -> usize {
        let remaining = &self.input[self.pos..];
        let n = remaining.len().min(len);
        buf[..n].copy_from_slice(&remaining[..n]);
        self.pos += n;
        n
    }
}

/// A [`Refiller`] backed by any [`std::io::Read`] source — files, stdin, sockets, …
///
/// I/O errors are silently treated as end-of-input. If you need error
/// propagation, wrap the reader in a type that converts errors to EOF before
/// passing it here.
#[derive(Debug)]
pub struct ReadRefiller<R: std::io::Read + std::fmt::Debug> {
    reader: R,
}

impl<R: std::io::Read + std::fmt::Debug> ReadRefiller<R> {
    /// Creates a new `ReadRefiller` wrapping `reader`.
    pub fn new(reader: R) -> Self {
        Self { reader }
    }
}

impl<R: std::io::Read + std::fmt::Debug> Refiller for ReadRefiller<R> {
    fn refill(&mut self, buf: &mut [u8], len: usize) -> usize {
        self.reader.read(&mut buf[..len]).unwrap_or(0)
    }
}
