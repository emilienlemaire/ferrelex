//! # Ferrelex
//!
//! **Ferrelex** is a crate that enables you to create powerful unicode-friendly lexers.
//!
//! Per say ferrelex is a lexer generator, leveraging the power of proc macros to enable
//! you to write readable regexes and match against from your selected input.
//!
//! # Example
//!
//! ```rust
//! use ferrelex::{lexer::lex, lexbuf::{lexbuf::utf8::LexBuf, refiller::Utf8Refiller}};
//!
//! // The lex macro is where you regexes and pattern matcher live.
//! lex! {
//!
//!     // Regexes are const of type `Regex`, valued with the regex
//!     // expression you'd like to match
//!     const ASCII_LETTER: Regex = ('a'..'z') | ('A'..'Z');
//!     // Remember that some character do not fit in the rust character type, but you can still
//!     // match against them using strings.
//!     const LAMBDA: Regex = "λ";
//!
//!     // Your matcher lives inside a function, it must at least accept a `LexBuf` type as an
//!     // argument, you can also define your own LexBuf, with your prefered input type.
//!     // This function can do any computing you'd like, and accept any input you'd like.
//!     // The ouput type can be anything you'd like.
//!     // Here we use the provided `LexBuf` for UTF-8 strings.
//!     pub fn my_lexer(lexbuf: &mut LexBuf) -> bool {
//!         // The `#[lexer]` attributes must preced your match expression
//!         #[lexer]
//!         match lexbuf {
//!             ASCII_LETTER => true,
//!             // You must end your match expression with a catch all error case, which you
//!             // can handle the way you'd like.
//!             _ => false,
//!         }
//!     }
//! }
//!
//! fn main() {
//!     // Before using your lexing function you have to define a mutable lexbuf.
//!     let mut lexbuf = LexBuf::new(Utf8Refiller::new(String::from("hello")));
//!
//!     // You can then use your lexbuf to lex your input, every time a valid value for your lexer
//!     // is found, the lexbuf advance. In our example we have to take five steps to arrive at the
//!     // end of our matching string.
//!     assert!(my_lexer(&mut lexbuf));
//!     assert!(my_lexer(&mut lexbuf));
//!     assert!(my_lexer(&mut lexbuf));
//!     assert!(my_lexer(&mut lexbuf));
//!     assert!(my_lexer(&mut lexbuf));
//!     assert!(!my_lexer(&mut lexbuf));
//! }
//! ```

pub mod lexer {
    pub use ferrelex_macro::*;
}

pub mod lexbuf {
    pub use ferrelex_core::lexbuf;
    pub use ferrelex_core::refiller;
}

