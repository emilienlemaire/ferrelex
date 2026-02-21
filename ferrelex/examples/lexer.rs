extern crate ferrelex;

use ferrelex::{lexbuf::{lexbuf::utf8::LexBuf, refiller::Utf8Refiller}, lexer};

#[derive(Debug)]
enum Token {
    Ascii(String),
    Lambda(String),
    Invalid
}

lexer::lex!{
    const ASCII_LETTERS: Regex = ('a'..'z') | ('A'..'Z');
    const LAMBDA: Regex = "λ";
    pub fn lex(lexbuf: &mut LexBuf) -> Token {
        #[lexer]
        match lexbuf {
            ASCII_LETTERS => Token::Ascii(lexbuf.lexeme().unwrap()) ,
            LAMBDA => Token::Lambda(lexbuf.lexeme().unwrap()),
            _ => Token::Invalid
        }
    }
}

fn main() {
    let mut lexbuf = LexBuf::new(Utf8Refiller::new(String::from("λhello")));
    dbg!(lex(&mut lexbuf));
    dbg!(lex(&mut lexbuf));
    dbg!(lex(&mut lexbuf));
    dbg!(lex(&mut lexbuf));
    dbg!(lex(&mut lexbuf));
    dbg!(lex(&mut lexbuf));
    dbg!(lex(&mut lexbuf));
    dbg!(lex(&mut lexbuf));
    dbg!(lex(&mut lexbuf));
    dbg!(lex(&mut lexbuf));
    dbg!(lex(&mut lexbuf));
    dbg!(lex(&mut lexbuf));
}
