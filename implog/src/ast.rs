use string_interner::StringInterner;
use string_interner::backend::StringBackend;
use string_interner::symbol::SymbolU16;

use crate::table::Value;

pub type Symbol = SymbolU16;
pub type NameInterner = StringInterner<StringBackend<Symbol>>;

#[derive(Debug)]
pub enum StatementAST {
    Rule(RuleAST),
    Question(Vec<LiteralAST>),
}

#[derive(Debug)]
pub struct RuleAST {
    pub head: LiteralAST,
    pub body: Vec<LiteralAST>,
}

#[derive(Debug)]
pub struct LiteralAST {
    pub lhs: Vec<AtomAST>,
    pub rhs: AtomAST,
}

#[derive(Debug)]
pub struct AtomAST {
    pub relation: Symbol,
    pub terms: Vec<TermAST>,
}

#[derive(Debug)]
pub enum TermAST {
    Variable(Symbol),
    Constant(Value),
}
