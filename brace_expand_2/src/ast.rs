use crate::tokenizer::Token;


#[derive(Debug, Clone, PartialEq)]
pub enum AstItem {
    Leaf(String),
    Choices(Vec<Ast>),
}

// The items in an Ast should always alternate between Leafs and Choices
// (since consecutive Terms in a Pattern can and should be combined)
pub type Ast = Vec<AstItem>;


/// Creates an AstItem::Choices from the start of the provided token
/// slice, which should begin immediately after the OpenBrace. Stops
/// when it reaches a CloseBrace.
///
/// Returns the AST item and the number of tokens that were consumed.
fn choices_from_tokens_partial(tokens: &[Token]) -> (AstItem, usize) {
    let mut v = Vec::new();

    let mut i = 0;
    while i < tokens.len() {
        let (ast, ast_size) = ast_from_tokens_partial(&tokens[i..]);
        v.push(ast);
        i += ast_size;

        if let Some(Token::CloseBrace) = tokens.get(i) {
            break;
        }

        if let Some(Token::Comma) = tokens.get(i) {
            i += 1;
        }
    }

    (AstItem::Choices(v), i)
}


/// Creates an Ast from the start of the provided token slice.
/// Stops when it reaches a CloseBrace or Comma.
///
/// Returns the AST and the number of tokens that were consumed.
fn ast_from_tokens_partial(tokens: &[Token]) -> (Ast, usize) {
    let mut pat = Ast::new();

    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::OpenBrace => {
                let (new_item, new_item_size) = choices_from_tokens_partial(&tokens[i+1..]);
                pat.push(new_item);
                i += 1 + new_item_size;
            },
            Token::CloseBrace => break,
            Token::Comma => break,
            Token::Term(s) => pat.push(AstItem::Leaf(s.to_owned())),
        }
        i += 1;
    }

    (pat, i)
}


/// Converts a slice of Tokens to an AST.
pub fn ast_from_tokens(tokens: &[Token]) -> Result<Ast, String> {
    let (ast, amt_consumed) = ast_from_tokens_partial(tokens);

    if amt_consumed < tokens.len() {
        Err(format!("unexpected {:?} at position {}", tokens[amt_consumed], amt_consumed))
    } else {
        Ok(ast)
    }
}


fn ast_item_max_expansion_length(item: &AstItem) -> usize {
    match item {
        AstItem::Leaf(s) => s.len(),
        AstItem::Choices(v) =>
            v.iter().map(ast_max_expansion_length).max().unwrap_or(0),
    }
}


/// Calculates the length of the longest string this AST will evaluate
/// to.
pub fn ast_max_expansion_length(ast: &Ast) -> usize {
    ast.iter().map(ast_item_max_expansion_length).sum()
}


fn ast_item_num_expansions(item: &AstItem) -> usize {
    match item {
        AstItem::Leaf(_) => 1,
        AstItem::Choices(v) =>
            v.iter().map(ast_num_expansions).sum(),
    }
}


/// Calculates the total number of expansions this AST will evaluate to.
pub fn ast_num_expansions(ast: &Ast) -> usize {
    ast.iter().map(ast_item_num_expansions).product()
}


#[cfg(test)]
mod tests {
    use super::*;

    use crate::tokenizer::tokenize;

    #[test]
    fn test_simple_expansion_in_middle_of_string() {
        let tokens = tokenize("a{b,c}d", true);
        let ast = ast_from_tokens(&tokens);

        assert_eq!(ast, Ok(vec![
            AstItem::Leaf("a".to_owned()),
            AstItem::Choices(vec![
                vec![AstItem::Leaf("b".to_owned())],
                vec![AstItem::Leaf("c".to_owned())],
            ]),
            AstItem::Leaf("d".to_owned()),
        ]));
    }

    #[test]
    fn test_nested_expansion() {
        let tokens = tokenize("{a,b}c{e,f{g,h}}", true);
        let ast = ast_from_tokens(&tokens);

        assert_eq!(ast, Ok(vec![
            AstItem::Choices(vec![
                vec![AstItem::Leaf("a".to_owned())],
                vec![AstItem::Leaf("b".to_owned())],
            ]),
            AstItem::Leaf("c".to_owned()),
            AstItem::Choices(vec![
                vec![AstItem::Leaf("e".to_owned())],
                vec![
                    AstItem::Leaf("f".to_owned()),
                    AstItem::Choices(vec![
                        vec![AstItem::Leaf("g".to_owned())],
                        vec![AstItem::Leaf("h".to_owned())],
                    ]),
                ],
            ]),
        ]));
    }

    #[test]
    fn test_empty_terms() {
        let tokens = tokenize("a{,b,,c,}d", true);
        let ast = ast_from_tokens(&tokens);

        assert_eq!(ast, Ok(vec![
            AstItem::Leaf("a".to_owned()),
            AstItem::Choices(vec![
                vec![],
                vec![AstItem::Leaf("b".to_owned())],
                vec![],
                vec![AstItem::Leaf("c".to_owned())],
                vec![],
            ]),
            AstItem::Leaf("d".to_owned()),
        ]));
    }

    #[test]
    fn test_simple_max_expansion_length() {
        let tokens = tokenize("a{b,c}d", true);
        let ast = ast_from_tokens(&tokens).unwrap();
        assert_eq!(ast_max_expansion_length(&ast), 3);
    }

    #[test]
    fn test_nested_max_expansion_length() {
        let tokens = tokenize("{a,b}c{e,f{g,h}}", true);
        let ast = ast_from_tokens(&tokens).unwrap();
        assert_eq!(ast_max_expansion_length(&ast), 4);
    }

    #[test]
    fn test_max_expansion_length_with_empty_terms() {
        let tokens = tokenize("a{,b,,c,}d{}", true);
        let ast = ast_from_tokens(&tokens).unwrap();
        assert_eq!(ast_max_expansion_length(&ast), 3);
    }

    #[test]
    fn test_simple_num_expansions() {
        let tokens = tokenize("a{b,c}d", true);
        let ast = ast_from_tokens(&tokens).unwrap();
        assert_eq!(ast_num_expansions(&ast), 2);
    }

    #[test]
    fn test_nested_num_expansions() {
        let tokens = tokenize("{a,b}c{e,f{g,h}}", true);
        let ast = ast_from_tokens(&tokens).unwrap();
        assert_eq!(ast_num_expansions(&ast), 6);
    }

    #[test]
    fn test_num_expansions_with_empty_terms() {
        let tokens = tokenize("a{,b,,c,}d{}", true);
        let ast = ast_from_tokens(&tokens).unwrap();
        assert_eq!(ast_num_expansions(&ast), 5);
    }
}
