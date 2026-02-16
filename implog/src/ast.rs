use std::collections::BTreeSet;

use string_interner::StringInterner;
use string_interner::backend::StringBackend;
use string_interner::symbol::SymbolU16;

use crate::table::Value;

pub type Symbol = SymbolU16;
pub type NameInterner = StringInterner<StringBackend<Symbol>>;

#[derive(Debug, Clone)]
pub enum StatementAST {
    Rule(RuleAST),
    Question(Vec<AtomAST>),
}

#[derive(Debug, Clone)]
pub struct RuleAST {
    pub head: AtomAST,
    pub speculate: bool,
    pub body: Vec<LiteralAST>,
}

#[derive(Debug, Clone)]
pub struct LiteralAST {
    pub lhs: Vec<AtomAST>,
    pub rhs: AtomAST,
}

#[derive(Debug, Clone)]
pub struct AtomAST {
    pub relation: Symbol,
    pub terms: Vec<TermAST>,
}

#[derive(Debug, Clone, Copy)]
pub enum TermAST {
    Variable(Symbol),
    Constant(Value),
}

pub fn check_range_restricted(stmt: &StatementAST) -> bool {
    let range = match &stmt {
        StatementAST::Rule(rule) => collect_variable_range_literals(&rule.body),
        StatementAST::Question(body) => collect_variable_range_atoms(body),
    };

    if let StatementAST::Rule(rule) = &stmt {
        if !is_range_restricted(&rule.head, &range) {
            return false;
        }

        for literal in &rule.body {
            if !literal
                .lhs
                .iter()
                .all(|atom| is_range_restricted(atom, &range))
            {
                return false;
            }
        }
    }

    true
}

fn collect_variable_range_literals(body: &[LiteralAST]) -> BTreeSet<Symbol> {
    let mut symbols = BTreeSet::new();
    for literal in body {
        for term in &literal.rhs.terms {
            if let TermAST::Variable(symbol) = term {
                symbols.insert(*symbol);
            }
        }
    }
    symbols
}

fn collect_variable_range_atoms(body: &[AtomAST]) -> BTreeSet<Symbol> {
    let mut symbols = BTreeSet::new();
    for atom in body {
        for term in &atom.terms {
            if let TermAST::Variable(symbol) = term {
                symbols.insert(*symbol);
            }
        }
    }
    symbols
}

fn is_range_restricted(atom: &AtomAST, range: &BTreeSet<Symbol>) -> bool {
    for term in &atom.terms {
        if let TermAST::Variable(symbol) = term
            && !range.contains(symbol)
        {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use crate::grammar::StatementParser;

    use super::*;

    #[test]
    fn test_range_restricted() {
        let range_restricted = [
            "? .",
            "? A(x, y), B(y, z).",
            "? A(x, y), B(y, z).",
            "C(x, y, z) :- A(x, y), B(y, z).",
            "[C(x, z)] :- A(x, y), B(y, z).",
        ];
        let not_range_restricted = [
            "D(x) :- C(x, y, z) -> A(x, y).",
            "C(x, y, z) :- A(x, y).",
        ];
        let mut interner = NameInterner::new();

        for stmt in &range_restricted {
            let stmt = StatementParser::new().parse(&mut interner, stmt).unwrap();
            assert!(check_range_restricted(&stmt));
        }
        for stmt in &not_range_restricted {
            let stmt = StatementParser::new().parse(&mut interner, stmt).unwrap();
            assert!(!check_range_restricted(&stmt));
        }
    }
}
