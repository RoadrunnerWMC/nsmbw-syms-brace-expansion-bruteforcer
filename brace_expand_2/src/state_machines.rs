use crate::ast::{Ast, AstItem};


pub trait StateMachine {
    /// Resets the state machine and all of its children to their
    /// initial states.
    fn reset(&mut self);

    /// Appends the string representation of the current state to a
    /// provided String.
    fn fill(&self, target: &mut String);

    /// Advances to the next state. Returns true if the state we
    /// advanced to is valid, false otherwise. When the state becomes
    /// invalid, you'll need to reset() in order to iterate again.
    fn advance(&mut self) -> bool;
}

#[derive(Debug)]
struct AstLeafItemStateMachine {
    contents: String,
    valid: bool,
}

impl AstLeafItemStateMachine {
    fn new(contents: &str) -> Self {
        Self{contents: contents.to_owned(), valid: true}
    }
}

impl StateMachine for AstLeafItemStateMachine {
    fn reset(&mut self) {
        self.valid = true;
    }

    fn fill(&self, target: &mut String) {
        if self.valid {
            target.push_str(&self.contents)
        }
    }

    fn advance(&mut self) -> bool {
        self.valid = false;
        false
    }
}

#[derive(Debug)]
struct AstChoicesItemStateMachine {
    children: Vec<AstStateMachine>,
    current_index: usize,
}

impl AstChoicesItemStateMachine {
    fn new(choices: &[Ast]) -> Self {
        Self{children: choices.iter().map(AstStateMachine::new).collect(), current_index: 0}
    }
}

impl StateMachine for AstChoicesItemStateMachine {
    fn reset(&mut self) {
        for it in &mut self.children {
            it.reset();
        }
        self.current_index = 0;
    }

    fn fill(&self, target: &mut String) {
        if self.current_index < self.children.len() {
            self.children[self.current_index].fill(target);
        }
    }

    fn advance(&mut self) -> bool {
        if self.current_index >= self.children.len() {
            return false;
        }
        if self.children[self.current_index].advance() {
            return true;
        }
        self.current_index += 1;
        self.current_index < self.children.len()
    }
}

#[derive(Debug)]
enum AstItemStateMachine {
    Leaf(AstLeafItemStateMachine),
    Choices(AstChoicesItemStateMachine),
}

impl AstItemStateMachine {
    fn new(item: &AstItem) -> Self {
        match item {
            AstItem::Leaf(s) => Self::Leaf(AstLeafItemStateMachine::new(s)),
            AstItem::Choices(v) => Self::Choices(AstChoicesItemStateMachine::new(v))
        }
    }
}

impl StateMachine for AstItemStateMachine {
    fn reset(&mut self) {
        match self {
            Self::Leaf(sm) => sm.reset(),
            Self::Choices(sm) => sm.reset(),
        }
    }

    fn fill(&self, target: &mut String) {
        match self {
            Self::Leaf(sm) => sm.fill(target),
            Self::Choices(sm) => sm.fill(target),
        }
    }

    fn advance(&mut self) -> bool {
        match self {
            Self::Leaf(sm) => sm.advance(),
            Self::Choices(sm) => sm.advance(),
        }
    }
}

#[derive(Debug)]
pub struct AstStateMachine {
    children: Vec<AstItemStateMachine>,
}

impl AstStateMachine {
    pub fn new(ast: &Ast) -> Self {
        Self{children: ast.iter().map(AstItemStateMachine::new).collect()}
    }
}

impl StateMachine for AstStateMachine {
    fn reset(&mut self) {
        for it in &mut self.children {
            it.reset();
        }
    }

    fn fill(&self, target: &mut String) {
        for child in &self.children {
            child.fill(target);
        }
    }

    fn advance(&mut self) -> bool {
        for child in &mut self.children.iter_mut().rev() {
            if child.advance() {
                return true;
            } else {
                child.reset();
            }
        }
        false
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    use crate::tokenizer::tokenize;
    use crate::ast::ast_from_tokens;

    #[test]
    fn test_simple_expansion_in_middle_of_string() {
        let tokens = tokenize("a{b,c}d", true);
        let ast = ast_from_tokens(&tokens).unwrap();
        let mut sm = AstStateMachine::new(&ast);

        let mut s = String::new();

        sm.fill(&mut s);
        assert_eq!(&s, "abd");
        assert!(sm.advance());

        s.clear();
        sm.fill(&mut s);
        assert_eq!(&s, "acd");
        assert!(!sm.advance());
    }

    #[test]
    fn test_nested_expansion() {
        let tokens = tokenize("{a,b}c{e,f{g,h}}", true);
        let ast = ast_from_tokens(&tokens).unwrap();
        let mut sm = AstStateMachine::new(&ast);

        let mut s = String::new();

        sm.fill(&mut s);
        assert_eq!(&s, "ace");
        assert!(sm.advance());

        s.clear();
        sm.fill(&mut s);
        assert_eq!(&s, "acfg");
        assert!(sm.advance());

        s.clear();
        sm.fill(&mut s);
        assert_eq!(&s, "acfh");
        assert!(sm.advance());

        s.clear();
        sm.fill(&mut s);
        assert_eq!(&s, "bce");
        assert!(sm.advance());

        s.clear();
        sm.fill(&mut s);
        assert_eq!(&s, "bcfg");
        assert!(sm.advance());

        s.clear();
        sm.fill(&mut s);
        assert_eq!(&s, "bcfh");
        assert!(!sm.advance());
    }

    #[test]
    fn test_empty_terms() {
        let tokens = tokenize("a{,b,,c,}d", true);
        let ast = ast_from_tokens(&tokens).unwrap();
        let mut sm = AstStateMachine::new(&ast);

        let mut s = String::new();

        sm.fill(&mut s);
        assert_eq!(&s, "ad");
        assert!(sm.advance());

        s.clear();
        sm.fill(&mut s);
        assert_eq!(&s, "abd");
        assert!(sm.advance());

        s.clear();
        sm.fill(&mut s);
        assert_eq!(&s, "ad");
        assert!(sm.advance());

        s.clear();
        sm.fill(&mut s);
        assert_eq!(&s, "acd");
        assert!(sm.advance());

        s.clear();
        sm.fill(&mut s);
        assert_eq!(&s, "ad");
        assert!(!sm.advance());
    }
}
