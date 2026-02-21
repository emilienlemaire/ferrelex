use std::{path::PathBuf, string::FromUtf8Error};

use crate::refiller::Refiller;

#[derive(Debug)]
struct Utf8DecodeError {
    pub byte_pos_in_buf: usize,
    pub bad_byte: u8,
}

#[derive(Debug, Default, Clone, Copy)]
struct Pos {
    offset: usize,
    bytes_offset: usize,
    pos: usize,
    bytes_pos: usize,
    bol: usize,
    bytes_bol: usize,
    line: usize,
}

pub struct LexBuf<R: Refiller> {
    buf: Vec<u8>,
    len: usize,
    curr_pos: Pos,
    start_pos: Pos,
    marked_pos: Pos,
    marked_val: isize,
    chunk_size: usize,
    filename: PathBuf,
    finished: bool,
    refiller: R,
}

impl<R: Refiller + std::fmt::Debug> std::fmt::Debug for LexBuf<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LexBuf")
            .field("len", &self.len)
            .field("curr_pos", &self.curr_pos)
            .field("start_pos", &self.start_pos)
            .field("marked_pos", &self.marked_pos)
            .field("marked_val", &self.marked_val)
            .field("chunk_size", &self.chunk_size)
            .field("filename", &self.filename)
            .field("finished", &self.finished)
            .field("refiller", &self.refiller)
            .finish()
    }
}

impl<R: Refiller> LexBuf<R>
where
    R: std::fmt::Debug,
{
    pub fn new(refiller: R) -> Self {
        Self::with_chunk_size(refiller, 512)
    }

    pub fn with_chunk_size(refiller: R, chunk_size: usize) -> Self {
        let cap = chunk_size.saturating_add(4).max(4);
        Self {
            buf: vec![0u8; cap],
            len: 0,
            curr_pos: Default::default(),
            start_pos: Default::default(),
            marked_pos: Default::default(),
            marked_val: 0,
            filename: PathBuf::from(""),
            chunk_size,
            finished: false,
            refiller,
        }
    }

    pub fn mark(&mut self, marked_val: isize) {
        self.marked_pos = self.curr_pos;
        self.marked_val = marked_val;
    }

    pub fn start(&mut self) {
        self.start_pos = self.curr_pos;
        self.mark(-1);
    }

    pub fn backtrack(&mut self) -> isize {
        self.curr_pos = self.marked_pos;
        self.marked_val
    }

    fn refill(&mut self) {
        if self.len + self.chunk_size > self.buf.len() {
            let start = self.start_pos.pos;
            let start_bytes = self.start_pos.bytes_pos;
            let len_from_start = self.len.saturating_sub(start_bytes);
            if len_from_start + self.chunk_size <= self.buf.len() {
                self.buf.copy_within(start_bytes..start_bytes + len_from_start, 0);
            } else {
                let new_len = (self.buf.len() + self.chunk_size) * 2;
                self.buf.resize(new_len, 0);
                self.buf.copy_within(start_bytes..start_bytes + len_from_start, 0);
            }
            self.len = len_from_start;
            self.curr_pos.offset += start;
            self.curr_pos.bytes_offset += start_bytes;
            self.curr_pos.pos = self.curr_pos.pos.saturating_sub(start);
            self.curr_pos.bytes_pos = self.curr_pos.bytes_pos.saturating_sub(start_bytes);
            self.marked_pos.pos = self.marked_pos.pos.saturating_sub(start);
            self.marked_pos.bytes_pos = self.marked_pos.bytes_pos.saturating_sub(start_bytes);
            self.start_pos.pos = 0;
            self.start_pos.bytes_pos = 0;
        }
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

    pub fn new_line(&mut self) {
        self.curr_pos.line += 1;
        self.curr_pos.bol = self.curr_pos.pos + self.curr_pos.offset;
        self.curr_pos.bytes_bol = self.curr_pos.bytes_pos + self.curr_pos.bytes_offset;
    }

    #[inline]
    fn decode_next_char_utf8(&mut self) -> Option<Result<char, Utf8DecodeError>> {
        if self.curr_pos.bytes_pos >= self.len {
            return None;
        }

        let p = self.curr_pos.bytes_pos;
        let b0 = self.buf[p];

        // ASCII fast path
        if b0 < 0x80 {
            self.curr_pos.bytes_pos += 1;
            self.curr_pos.pos += 1;
            return Some(Ok(b0 as char));
        }

        let width = if (b0 & 0b1110_0000) == 0b1100_0000 {
            2
        } else if (b0 & 0b1111_0000) == 0b1110_0000 {
            3
        } else if (b0 & 0b1111_1000) == 0b1111_0000 {
            4
        } else {
            return Some(Err(Utf8DecodeError {
                byte_pos_in_buf: p,
                bad_byte: b0,
            }));
        };

        if self.len - p < width {
            return None;
        }

        let cp: u32;
        match width {
            2 => {
                let b1 = self.buf[p + 1];
                if (b1 & 0b1100_0000) != 0b1000_0000 {
                    return Some(Err(Utf8DecodeError {
                        byte_pos_in_buf: p,
                        bad_byte: b0,
                    }));
                }
                cp = ((b0 & 0b0001_1111) as u32) << 6 | ((b1 & 0b0011_1111) as u32);
                if cp < 0x80 {
                    return Some(Err(Utf8DecodeError {
                        byte_pos_in_buf: p,
                        bad_byte: b0,
                    }));
                }
            }
            3 => {
                let b1 = self.buf[p + 1];
                let b2 = self.buf[p + 2];
                if (b1 & 0b1100_0000) != 0b1000_0000 || (b2 & 0b1100_0000) != 0b1000_0000 {
                    return Some(Err(Utf8DecodeError {
                        byte_pos_in_buf: p,
                        bad_byte: b0,
                    }));
                }
                cp = ((b0 & 0b0000_1111) as u32) << 12
                    | ((b1 & 0b0011_1111) as u32) << 6
                    | ((b2 & 0b0011_1111) as u32);
                if cp < 0x800 {
                    return Some(Err(Utf8DecodeError {
                        byte_pos_in_buf: p,
                        bad_byte: b0,
                    }));
                }
            }
            4 => {
                let b1 = self.buf[p + 1];
                let b2 = self.buf[p + 2];
                let b3 = self.buf[p + 3];
                if (b1 & 0b1100_0000) != 0b1000_0000
                    || (b2 & 0b1100_0000) != 0b1000_0000
                    || (b3 & 0b1100_0000) != 0b1000_0000
                {
                    return Some(Err(Utf8DecodeError {
                        byte_pos_in_buf: p,
                        bad_byte: b0,
                    }));
                }
                cp = ((b0 & 0b0000_0111) as u32) << 18
                    | ((b1 & 0b0011_1111) as u32) << 12
                    | ((b2 & 0b0011_1111) as u32) << 6
                    | ((b3 & 0b0011_1111) as u32);
                if cp < 0x10000 {
                    return Some(Err(Utf8DecodeError {
                        byte_pos_in_buf: p,
                        bad_byte: b0,
                    }));
                }
            }
            _ => unreachable!(),
        };

        // Reject invalid scalar values
        if cp > 0x10FFFF || (0xD800..=0xDFFF).contains(&cp) {
            return Some(Err(Utf8DecodeError {
                byte_pos_in_buf: p,
                bad_byte: b0,
            }));
        }

        let ch = unsafe { char::from_u32_unchecked(cp) };
        self.curr_pos.bytes_pos += width;
        self.curr_pos.pos += 1;
        Some(Ok(ch))
    }

    pub fn next_int(&mut self) -> isize {
        loop {
            if !self.finished && self.curr_pos.bytes_pos == self.len {
                self.refill();
            }
            if self.finished && self.curr_pos.bytes_pos == self.len {
                break -1;
            } else {
                match self.decode_next_char_utf8() {
                    None => {
                        if self.finished {
                            break -1;
                        }
                        self.refill();
                    }
                    Some(Ok(ch)) => {
                        if ch == '\n' {
                            self.new_line();
                        }
                        break ch as isize;
                    }
                    Some(Err(_)) => {
                        break -1;
                    }
                }
            }
        }
    }

    pub fn next(&mut self) -> Option<char> {
        loop {
            if !self.finished && self.curr_pos.bytes_pos == self.len {
                self.refill();
            }
            if self.finished && self.curr_pos.bytes_pos == self.len {
                break None;
            } else {
                match self.decode_next_char_utf8() {
                    None => {
                        self.refill();
                    }
                    Some(Ok(ch)) => {
                        if ch == '\n' {
                            self.new_line();
                        }
                        break Some(ch);
                    }
                    Some(Err(_)) => {
                        break None;
                    }
                }
            }
        }
    }

    pub fn lexeme(&self) -> Result<String, FromUtf8Error> {
        String::from_utf8(
            self.buf[self.start_pos.bytes_pos..self.curr_pos.bytes_pos].to_vec(),
        )
    }
}

pub mod utf8 {
    use crate::refiller::Utf8Refiller;

    pub type LexBuf = super::LexBuf<Utf8Refiller>;
}
