use std::collections::BTreeMap;

use crate::ast::{AtomAST, LiteralAST, StatementAST};
use crate::representation::{Assumption, Symbol, Table};

pub struct Environment<A: Assumption> {
    tables: BTreeMap<String, Table<A>>,
    log: String,
}

impl<A: Assumption> Environment<A> {
    pub fn new() -> Self {
        Self {
            tables: BTreeMap::new(),
            log: String::new(),
        }
    }

    pub fn interpret(&mut self, stmts: &[StatementAST]) -> &str {
        self.log.clear();
        let mut rules = vec![];

        for stmt in stmts {
            match stmt {
                StatementAST::Rule(head, body) => {
                    self.register_table_for_atom(head);
                    for atom in body {
                        self.register_table_for_atom(atom);
                    }
                    rules.push((head, body));
                },
                StatementAST::Question(body) => {
                    for atom in body {
                        self.register_table_for_atom(atom);
                    }
                    self.interpret_rules(&rules);
                    self.interpret_question(body);
                },
            }
        }

        &self.log
    }

    fn table(&self, relation: &str) -> &Table<A> {
        self.tables.get(relation).unwrap()
    }

    fn table_mut(&mut self, relation: &str) -> &mut Table<A> {
        self.tables.get_mut(relation).unwrap()
    }

    fn register_table_for_atom(&mut self, atom: &AtomAST) {
        match atom {
            AtomAST::Literal(lit) | AtomAST::Brackets(lit) => self.register_table_for_literal(lit),
            AtomAST::Arrow(lit1, lit2) => {
                self.register_table_for_literal(lit1);
                self.register_table_for_literal(lit2);
            }
        }
    }

    fn register_table_for_literal(&mut self, lit: &LiteralAST) {
        if !self.tables.contains_key(&lit.relation) {
            self.tables.insert(lit.relation.clone(), Table::new());
        }
    }

    fn interpret_rules(&mut self, rules: &[(&AtomAST, &Vec<AtomAST>)]) {

    }

    fn interpret_question(&mut self, question: &[AtomAST]) {

    }
}
