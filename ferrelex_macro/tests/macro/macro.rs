use ferrelex_macro::lex;

lex! {
    const ASCII_LETTERS: Regex = ('a'..'z') | ('A'..'Z');
    struct Hello;
    pub fn lex(lexbuf: ferrelex::Lexbuf) {
        #[lexer]
        match lexbuf {
            ASCII_LETTER => (),
        }
    }
}

fn main() {}

