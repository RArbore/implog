use std::collections::BTreeSet;

use either::Either;

// Arbitrary type used for ground symbols. For now, should implement Copy.
pub type Symbol = i64;

#[derive(Debug, Clone)]
pub enum StatementAST {
    Rule(AtomAST, Vec<AtomAST>),
    Question(Vec<AtomAST>),
}

#[derive(Debug, Clone)]
pub enum AtomAST {
    Literal(LiteralAST),
    Brackets(LiteralAST),
    Arrow(LiteralAST, LiteralAST),
}

#[derive(Debug, Clone)]
pub struct LiteralAST {
    pub relation: String,
    pub terms: Vec<TermAST>,
}

#[derive(Debug, Clone)]
pub enum TermAST {
    Variable(String),
    Constant(Symbol),
}

impl StatementAST {
    pub fn head(&self) -> Option<&AtomAST> {
        use StatementAST::*;
        match self {
            Rule(head, _) => Some(head),
            Question(_) => None,
        }
    }

    pub fn body(&self) -> &Vec<AtomAST> {
        use StatementAST::*;
        match self {
            Rule(_, body) | Question(body) => body,
        }
    }
}

impl AtomAST {
    pub fn vars(&self) -> impl Iterator<Item = &str> + '_ {
        use AtomAST::*;
        // Either needed since iterators are different concrete types, even if both implement the
        // same Iterator<Item = &str> trait.
        match self {
            Literal(lit) | Brackets(lit) => Either::Left(lit.vars()),
            Arrow(lit1, lit2) => Either::Right(lit1.vars().chain(lit2.vars())),
        }
    }
}

impl LiteralAST {
    pub fn vars(&self) -> impl Iterator<Item = &str> + '_ {
        self.terms.iter().filter_map(TermAST::try_var)
    }
}

impl TermAST {
    pub fn try_var(&self) -> Option<&str> {
        use TermAST::*;
        match self {
            Variable(s) => Some(s),
            Constant(_) => None,
        }
    }

    pub fn try_cons(&self) -> Option<Symbol> {
        use TermAST::*;
        match self {
            Variable(_) => None,
            Constant(s) => Some(*s),
        }
    }
}

pub fn check(stmt: &StatementAST) -> bool {
    use AtomAST::*;
    // Check that a parsed statement is well formed. Just return true/false for now.

    // 1. Statements must be properly range restricted. The range of a statement is the set of
    //    variables appearing in the body as (just) literals or in the RHS literal of arrow atoms.
    //    The set of variables in the head, in the LHS literal of arrow atoms, or in the literal of
    //    bracket atoms must be a subset of the range.
    let mut range = BTreeSet::new();
    for atom in stmt.body() {
        match atom {
            Literal(lit) | Arrow(_, lit) => range.extend(lit.vars()),
            Brackets(_) => {}
        }
    }

    if let Some(head) = stmt.head()
        && head.vars().any(|var| !range.contains(var))
    {
        return false;
    }
    for atom in stmt.body() {
        match atom {
            Brackets(lit) | Arrow(lit, _) if lit.vars().any(|var| !range.contains(var)) => {
                return false;
            }
            _ => {}
        }
    }

    // 2. (TEMPORARY) no arrows in the head for now. If we have bandwidth we can implement a
    //    transform to get rid of arrows in the head.
    if let Some(head) = stmt.head()
        && let Arrow(_, _) = head
    {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use crate::grammar::ProgramParser;

    use super::*;

    fn parse_and_check(program: &str) {
        let parsed = ProgramParser::new().parse(program).unwrap();
        for stmt in parsed {
            assert!(check(&stmt));
        }
    }

    fn parse_and_fail_check(program: &str) {
        let parsed = ProgramParser::new().parse(program).unwrap();
        for stmt in parsed {
            assert!(!check(&stmt));
        }
    }

    #[test]
    fn parse_and_check_path() {
        let program = r#"
E(1, 2) :- .
E(2, 3) :- .
E(3, 4) :- .

? E(1, 2).
? E(1, 4).

P(x, y) :- E(x, y).

? E(x, y), P(y, z).

P(x, z) :- E(x, y), P(y, z).

? E(x, y).
? P(x, y).
? P(1, 4).
? P(4, 1).
"#;
        parse_and_check(&program);
    }

    #[test]
    fn parse_and_check_basic_assume() {
        let program = r#"
Q() :- P().
G() :- P() -> Q().
X() :- .

? Q().
? G().
? P().
? X().

[P()] :- X().

? Q().
? G().
? P().
? X().
"#;
        parse_and_check(&program);
    }

    #[test]
    fn parse_and_check_tricky() {
        let program = r#"
[A()] :- .
P() :- A().
Q() :- A().
G() :- P() -> Q().

? A().
? P().
? Q().
? G().

X(a, b) :- X(a, b).
[X(1, 2)] :- .
Y(a, b) :- Y(b, a), Y(b, a).
[Y(3, 4)] :- .

? X(a, b).
? Y(a, b).
"#;
        parse_and_check(&program);
    }

    #[test]
    fn parse_and_fail_check_range_head() {
        let program = r#"
A(a, b) :- B(b).
"#;
        parse_and_fail_check(&program);
    }

    #[test]
    fn parse_and_fail_check_range_bracket() {
        let program = r#"
A(b) :- B(b), [C(b, a)].
"#;
        parse_and_fail_check(&program);
    }

    #[test]
    fn parse_and_fail_check_range_arrow() {
        let program = r#"
A(b) :- C(a, b) -> B(b).
"#;
        parse_and_fail_check(&program);
    }

    // Temporary (see check()).
    #[test]
    fn parse_and_fail_check_arrow_in_head() {
        let program = r#"
A(1) -> B(2) :- .
"#;
        parse_and_fail_check(&program);
    }
}
