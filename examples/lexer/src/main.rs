use ferrelex::lex;

lex!{
    const ASCII_LETTERS: Regex = ('a'..'z') | ('A'..'Z');
    pub fn lex(lexbuf: ferrelex::Lexbuf) {
        #[lexer]
        match lexbuf {
            ASCII_LETTERS => (),
            _ => ()
        }
    }
}

fn main() {
    println!("Hello, world!");
}
