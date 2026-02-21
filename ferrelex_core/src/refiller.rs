pub trait Refiller {
    /// When the lexer needs more characters, it will call the given function,
    /// giving it a slice of Uchars `buf`, a position `pos` and a code point
    /// count `len`. The function should put `len` code points or less in `buf`,
    /// starting at position `pos`, and return the number of characters provided.
    /// A return value of 0 means end of input.
    fn refill(&mut self, buf: &mut [u8], len: usize) -> usize;
    /// This function should return the number of bytes a char has.
    fn bytes_per_char(c: char) -> usize;
}


#[derive(Debug)]
pub struct Utf8Refiller {
    input: String,
    curr_byte: usize
}

impl Utf8Refiller {
    pub fn new(input: String) -> Self {
        Self { input, curr_byte: 0 }
    }
}

impl Refiller for Utf8Refiller {
    fn refill(&mut self, buf: &mut [u8], len: usize) -> usize {
        let bytes = &self.input.as_bytes()[self.curr_byte..];
        let read_len = if bytes.len() > len {
            len
        } else {
            bytes.len()
        };
        buf[..read_len].copy_from_slice(&bytes[..read_len]);
        self.curr_byte = self.curr_byte.saturating_add(read_len);
        read_len
    }

    fn bytes_per_char(c: char) -> usize {
        c.len_utf8()
    }
}
