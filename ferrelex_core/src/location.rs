// Copyright (c) 2025-2026 Emilien Lemaire <emilien.lem@icloud.com>
// SPDX-License-Identifier: LGPL-3.0-only
// Licensed under the GNU Lesser General Public License v3.0, with the
// ferrelex Generated Code Exception. See the LICENSE file at the root
// of this repository for the full license text and exception terms.

use std::{fmt, path::PathBuf};

/// A point in a source file: line, column, and filename.
///
/// Obtained from [`LexBuf::start_pos`](crate::lexbuf::LexBuf::start_pos) or
/// [`LexBuf::end_pos`](crate::lexbuf::LexBuf::end_pos) after a successful match.
///
/// `Display` formats as `file:line:col`, or `line:col` when no filename is set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Position {
    /// Line number, 1-indexed. Always `0` when `#[lexer(no_line_tracking)]` is used.
    pub line: usize,
    /// Column number, 0-indexed character offset from the start of the line.
    pub col: usize,
    /// Source filename. Empty if [`LexBuf::set_filename`](crate::lexbuf::LexBuf::set_filename)
    /// was never called.
    pub filename: PathBuf,
}

impl fmt::Display for Position {
    /// Formats as `file:line:col`, or `line:col` when the filename is empty.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.filename.as_os_str().is_empty() {
            write!(f, "{}:{}", self.line, self.col)
        } else {
            write!(f, "{}:{}:{}", self.filename.display(), self.line, self.col)
        }
    }
}

/// A source span: the start and end [`Position`] of a token.
///
/// Obtained from [`LexBuf::location`](crate::lexbuf::LexBuf::location) after a
/// successful match. `end` is the position of the character immediately **after**
/// the last character of the token (exclusive).
///
/// `Display` formats as `file:sl:sc-el:ec` when both positions share the same
/// filename, or `start - end` otherwise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    /// Start of the token (inclusive).
    pub start: Position,
    /// End of the token (exclusive — the character just past the last matched char).
    pub end: Position,
}

impl fmt::Display for Location {
    /// Formats as `file:sl:sc-el:ec` when start and end share the same file,
    /// or `start - end` when they differ.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.start.filename == self.end.filename {
            if self.start.filename.as_os_str().is_empty() {
                write!(
                    f,
                    "{}:{}-{}:{}",
                    self.start.line, self.start.col, self.end.line, self.end.col
                )
            } else {
                write!(
                    f,
                    "{}:{}:{}-{}:{}",
                    self.start.filename.display(),
                    self.start.line,
                    self.start.col,
                    self.end.line,
                    self.end.col
                )
            }
        } else {
            write!(f, "{} - {}", self.start, self.end)
        }
    }
}
