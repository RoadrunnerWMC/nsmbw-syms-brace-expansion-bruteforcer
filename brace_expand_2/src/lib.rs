mod ast;
mod state_machines;
mod tokenizer;

use std::error::Error;

use ast::{ast_from_tokens, ast_max_expansion_length, ast_num_expansions};
use state_machines::{AstStateMachine, StateMachine};
use tokenizer::tokenize;

// ---------------------------------------------------------------------

#[derive(Debug)]
pub struct BraceExpandIterator {
    state_machine: AstStateMachine,
    is_done: bool,
    length_hint: usize,
    num_expansions_hint: usize
}

impl BraceExpandIterator {
    fn new(state_machine: AstStateMachine, length_hint: usize, num_expansions_hint: usize) -> Self {
        Self{state_machine, is_done: false, length_hint, num_expansions_hint}
    }

    pub fn next_into(&mut self, output: &mut String) -> bool {
        if self.is_done {
            return false;
        }
        output.clear();
        self.state_machine.fill(output);
        self.is_done = !self.state_machine.advance();
        true
    }

    pub fn max_expansion_length(&self) -> usize {
        self.length_hint
    }

    pub fn num_expansions(&self) -> usize {
        self.num_expansions_hint
    }
}

impl Iterator for BraceExpandIterator {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_done {
            return None;
        }
        let mut output = String::new();
        self.state_machine.fill(&mut output);
        self.is_done = !self.state_machine.advance();
        Some(output)
    }
}

// TODO: proper error return type
pub fn brace_expand_iter(input: &str, escape: bool) -> Result<BraceExpandIterator, Box<dyn Error>> {
    let tokens = tokenize(input, escape);
    let ast = ast_from_tokens(&tokens)?;
    let size_hint = ast_max_expansion_length(&ast);
    let num_expansions_hint = ast_num_expansions(&ast);
    let sm = AstStateMachine::new(&ast);
    Ok(BraceExpandIterator::new(sm, size_hint, num_expansions_hint))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_expansion_in_middle_of_string() {
        let output: Vec<String> = brace_expand_iter("a{b,c}d", true).unwrap().collect();

        assert_eq!(output, vec!["abd", "acd"]);
    }

    #[test]
    fn test_simple_expansion_in_middle_of_string_zero_alloc() {
        let mut iter = brace_expand_iter("a{b,c}d", true).unwrap();
        let mut output = String::new();

        assert!(iter.next_into(&mut output));
        assert_eq!(&output, "abd");
        assert!(iter.next_into(&mut output));
        assert_eq!(&output, "acd");
        assert!(!iter.next_into(&mut output));
    }

    #[test]
    fn test_nested_expansion() {
        let output: Vec<String> = brace_expand_iter("{a,b}c{e,f{g,h}}", true).unwrap().collect();

        assert_eq!(output, vec!["ace", "acfg", "acfh", "bce", "bcfg", "bcfh"]);
    }

    #[test]
    fn test_nested_expansion_zero_alloc() {
        let mut iter = brace_expand_iter("{a,b}c{e,f{g,h}}", true).unwrap();
        let mut output = String::new();

        assert!(iter.next_into(&mut output));
        assert_eq!(&output, "ace");
        assert!(iter.next_into(&mut output));
        assert_eq!(&output, "acfg");
        assert!(iter.next_into(&mut output));
        assert_eq!(&output, "acfh");
        assert!(iter.next_into(&mut output));
        assert_eq!(&output, "bce");
        assert!(iter.next_into(&mut output));
        assert_eq!(&output, "bcfg");
        assert!(iter.next_into(&mut output));
        assert_eq!(&output, "bcfh");
        assert!(!iter.next_into(&mut output));
    }

    #[test]
    fn test_empty_terms() {
        let output: Vec<String> = brace_expand_iter("a{,b,,c,}d", true).unwrap().collect();

        assert_eq!(output, vec!["ad", "abd", "ad", "acd", "ad"]);
    }

    #[test]
    fn test_empty_terms_zero_alloc() {
        let mut iter = brace_expand_iter("a{,b,,c,}d", true).unwrap();
        let mut output = String::new();

        assert!(iter.next_into(&mut output));
        assert_eq!(&output, "ad");
        assert!(iter.next_into(&mut output));
        assert_eq!(&output, "abd");
        assert!(iter.next_into(&mut output));
        assert_eq!(&output, "ad");
        assert!(iter.next_into(&mut output));
        assert_eq!(&output, "acd");
        assert!(iter.next_into(&mut output));
        assert_eq!(&output, "ad");
        assert!(!iter.next_into(&mut output));
    }
}