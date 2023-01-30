
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    OpenBrace,
    CloseBrace,
    Comma,
    Term(String),
}


/// Converts a string slice to a Vec of Tokens.
///
/// If escape is true, you can use backslashes to escape any character,
/// such as braces and commas. If it's false, backslashes will just be
/// treated like any other character.
pub fn tokenize(pattern: &str, escape: bool) -> Vec<Token> {
    let mut tokens = Vec::new();

    let mut is_escape_seq = false;

    for c in pattern.chars() {
        if is_escape_seq {
            if let Some(Token::Term(s)) = tokens.last_mut() {
                s.push(c);
            } else {
                tokens.push(Token::Term(c.to_string()));
            }
            is_escape_seq = false;
        } else {
            match c {
                '{' => tokens.push(Token::OpenBrace),
                '}' => tokens.push(Token::CloseBrace),
                ',' => tokens.push(Token::Comma),
                _ => {
                    if escape && c == '\\' {
                        is_escape_seq = true;
                    } else if let Some(Token::Term(s)) = tokens.last_mut() {
                        s.push(c);
                    } else {
                        tokens.push(Token::Term(c.to_string()));
                    }
                }
            }
        }
    }

    tokens
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_expansion_in_middle_of_string() {
        let tokens = tokenize("a{b,c}d", true);

        assert_eq!(tokens, vec![
            Token::Term("a".to_owned()),
            Token::OpenBrace,
            Token::Term("b".to_owned()),
            Token::Comma,
            Token::Term("c".to_owned()),
            Token::CloseBrace,
            Token::Term("d".to_owned()),
        ]);
    }

    #[test]
    fn test_multi_char_terms() {
        let tokens = tokenize("abc{def,ghi}jkl", true);

        assert_eq!(tokens, vec![
            Token::Term("abc".to_owned()),
            Token::OpenBrace,
            Token::Term("def".to_owned()),
            Token::Comma,
            Token::Term("ghi".to_owned()),
            Token::CloseBrace,
            Token::Term("jkl".to_owned()),
        ]);
    }

    #[test]
    fn test_nested_expansion() {
        let tokens = tokenize("{a,b}c{e,f{g,h}}", true);

        assert_eq!(tokens, vec![
            Token::OpenBrace,
            Token::Term("a".to_owned()),
            Token::Comma,
            Token::Term("b".to_owned()),
            Token::CloseBrace,
            Token::Term("c".to_owned()),
            Token::OpenBrace,
            Token::Term("e".to_owned()),
            Token::Comma,
            Token::Term("f".to_owned()),
            Token::OpenBrace,
            Token::Term("g".to_owned()),
            Token::Comma,
            Token::Term("h".to_owned()),
            Token::CloseBrace,
            Token::CloseBrace,
        ]);
    }

    #[test]
    fn test_empty_terms() {
        let tokens = tokenize("a{,b,,c,}d", true);

        assert_eq!(tokens, vec![
            Token::Term("a".to_owned()),
            Token::OpenBrace,
            Token::Comma,
            Token::Term("b".to_owned()),
            Token::Comma,
            Token::Comma,
            Token::Term("c".to_owned()),
            Token::Comma,
            Token::CloseBrace,
            Token::Term("d".to_owned()),
        ]);
    }

    #[test]
    fn test_escaping_commas() {
        let tokens = tokenize("{a\\,,b\\,}c", true);

        assert_eq!(tokens, vec![
            Token::OpenBrace,
            Token::Term("a,".to_owned()),
            Token::Comma,
            Token::Term("b,".to_owned()),
            Token::CloseBrace,
            Token::Term("c".to_owned()),
        ]);
    }

    #[test]
    fn test_not_escaping_commas() {
        let tokens = tokenize("{a\\,,b\\,}c", false);

        assert_eq!(tokens, vec![
            Token::OpenBrace,
            Token::Term("a\\".to_owned()),
            Token::Comma,
            Token::Comma,
            Token::Term("b\\".to_owned()),
            Token::Comma,
            Token::CloseBrace,
            Token::Term("c".to_owned()),
        ]);
    }

    #[test]
    fn test_escaping_braces() {
        let tokens = tokenize("{\\{a,b\\},c}d", true);

        assert_eq!(tokens, vec![
            Token::OpenBrace,
            Token::Term("{a".to_owned()),
            Token::Comma,
            Token::Term("b}".to_owned()),
            Token::Comma,
            Token::Term("c".to_owned()),
            Token::CloseBrace,
            Token::Term("d".to_owned()),
        ]);
    }

    #[test]
    fn test_not_escaping_braces() {
        let tokens = tokenize("{\\{a,b\\},c}d", false);

        assert_eq!(tokens, vec![
            Token::OpenBrace,
            Token::Term("\\".to_owned()),
            Token::OpenBrace,
            Token::Term("a".to_owned()),
            Token::Comma,
            Token::Term("b\\".to_owned()),
            Token::CloseBrace,
            Token::Comma,
            Token::Term("c".to_owned()),
            Token::CloseBrace,
            Token::Term("d".to_owned()),
        ]);
    }

    #[test]
    fn test_escaping_backslashes() {
        let tokens = tokenize("{\\\\{a,b\\\\},c}d", true);

        assert_eq!(tokens, vec![
            Token::OpenBrace,
            Token::Term("\\".to_owned()),
            Token::OpenBrace,
            Token::Term("a".to_owned()),
            Token::Comma,
            Token::Term("b\\".to_owned()),
            Token::CloseBrace,
            Token::Comma,
            Token::Term("c".to_owned()),
            Token::CloseBrace,
            Token::Term("d".to_owned()),
        ]);
    }

    #[test]
    fn test_not_escaping_backslashes() {
        let tokens = tokenize("{\\\\{a,b\\\\},c}d", false);

        assert_eq!(tokens, vec![
            Token::OpenBrace,
            Token::Term("\\\\".to_owned()),
            Token::OpenBrace,
            Token::Term("a".to_owned()),
            Token::Comma,
            Token::Term("b\\\\".to_owned()),
            Token::CloseBrace,
            Token::Comma,
            Token::Term("c".to_owned()),
            Token::CloseBrace,
            Token::Term("d".to_owned()),
        ]);
    }
}
