use core::iter::once;
use std::collections::BTreeMap;

use crate::assumption::DNFAssumption;
use crate::ast::{LiteralAST, RuleAST, StatementAST, Symbol, TermAST};
use crate::interner::Interner;
use crate::table::{Rows, Table, Value};

pub struct Environment {
    tables: BTreeMap<Symbol, Table>,
    assumption_interner: Interner<DNFAssumption>,
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            tables: BTreeMap::new(),
            assumption_interner: Interner::new(),
        }
    }

    pub fn interpret(&mut self, stmts: &[StatementAST]) {
        for idx in 0..stmts.len() {
            let mut rules = vec![];
            match &stmts[idx] {
                StatementAST::Rule(rule) => {
                    for literal in once(&rule.head).chain(rule.body.iter()) {
                        for atom in literal.lhs.iter().chain(once(&literal.rhs)) {
                            let num_determinant = atom.terms.len();
                            if let Some(table) = self.tables.get(&atom.relation) {
                                assert_eq!(table.num_determinant(), num_determinant);
                            } else {
                                self.tables
                                    .insert(atom.relation, Table::new(num_determinant));
                            }
                        }
                    }
                    rules.push(rule);
                }
                StatementAST::Question(question) => {
                    self.interpret_rules(rules);
                    self.interpret_question(question);
                }
            }
        }
    }

    fn interpret_rules(&mut self, rules: Vec<&RuleAST>) {
        for (_, table) in self.tables.iter_mut() {
            table.reset_delta();
        }
        let orders: Vec<_> = rules.iter().map(|rule| Self::order(&rule.body)).collect();
        let inv_orders: Vec<BTreeMap<Symbol, usize>> = orders
            .iter()
            .map(|order| {
                order
                    .iter()
                    .enumerate()
                    .map(|(idx, symbol)| (*symbol, idx))
                    .collect()
            })
            .collect();
        let one_id = self.assumption_interner.intern(DNFAssumption::one());

        'outer: loop {
            let mut answers = vec![];
            for rule_idx in 0..rules.len() {
                let rule = &rules[rule_idx];
                let order = &orders[rule_idx];
                let answer = self.query(&rule.body, order);
                answers.push(answer);
            }

            for (_, table) in self.tables.iter_mut() {
                table.mark_delta();
            }

            for rule_idx in 0..rules.len() {
                let head = &rules[rule_idx].head;
                let inv_order = &inv_orders[rule_idx];
                let answer = &answers[rule_idx];

                let rhs_relation = head.rhs.relation;
                let mut rhs_scratch_row = vec![0; head.rhs.terms.len() + 1];
                for answer_idx in 0..answer.num_rows() {
                    let answer = answer.get_row(answer_idx);
                    for (term_idx, term) in head.rhs.terms.iter().enumerate() {
                        match term {
                            TermAST::Variable(symbol) => {
                                rhs_scratch_row[term_idx] = answer[inv_order[symbol]]
                            }
                            TermAST::Constant(value) => rhs_scratch_row[term_idx] = *value,
                        }
                    }
                    rhs_scratch_row[head.rhs.terms.len()] = one_id.into();
                    let table = self.tables.get_mut(&rhs_relation).unwrap();
                    table.insert(&rhs_scratch_row, &mut |_, _| one_id.into());
                }
            }

            for (_, table) in &self.tables {
                if table.changed() {
                    continue 'outer;
                }
            }
            break;
        }
    }

    fn interpret_question(&self, question: &Vec<LiteralAST>) {
        let order = Self::order(question);
        let answer = self.query(question, &order);
        println!("Num answers: {}", answer.num_rows());
    }

    fn order(query: &Vec<LiteralAST>) -> Vec<Symbol> {
        let mut order = vec![];
        for literal in query {
            for term in &literal.rhs.terms {
                if let TermAST::Variable(symbol) = term
                    && !order.contains(symbol)
                {
                    order.push(*symbol);
                }
            }
        }
        order
    }

    fn query(&self, query: &Vec<LiteralAST>, order: &[Symbol]) -> Rows {
        let mut rows = Rows::new(order.len() + query.len());
        let mut current = BTreeMap::new();
        let mut symbol_stack = vec![];
        self.query_helper(query, order, &mut rows, &mut current, &mut symbol_stack);
        rows
    }

    fn query_helper(
        &self,
        query: &[LiteralAST],
        order: &[Symbol],
        rows: &mut Rows,
        current: &mut BTreeMap<Symbol, Value>,
        symbol_stack: &mut Vec<Symbol>,
    ) {
        if query.is_empty() {
            let row_id = rows.alloc_row();
            let row = rows.get_row_mut(row_id);
            for (idx, var) in order.into_iter().enumerate() {
                row[idx] = current[var];
            }
            return;
        }

        let literal = &query[0];
        let rest = &query[1..];
        let rhs_table = &self.tables[&literal.rhs.relation];
        assert_eq!(rhs_table.num_determinant(), literal.rhs.terms.len());
        let symbol_stack_head = symbol_stack.len();

        'outer: for (row, _) in rhs_table.rows(false) {
            for symbol_idx in symbol_stack_head..symbol_stack.len() {
                current.remove(&symbol_stack[symbol_idx]);
            }
            symbol_stack.truncate(symbol_stack_head);

            for col_idx in 0..rhs_table.num_determinant() {
                let in_row = row[col_idx];
                match literal.rhs.terms[col_idx] {
                    TermAST::Variable(var) => {
                        if let Some(old) = current.insert(var, in_row) {
                            if old != in_row {
                                continue 'outer;
                            }
                        } else {
                            symbol_stack.push(var);
                        }
                    }
                    TermAST::Constant(value) => {
                        if value != in_row {
                            continue 'outer;
                        }
                    }
                }
            }

            self.query_helper(rest, order, rows, current, symbol_stack);
        }

        for symbol_idx in symbol_stack_head..symbol_stack.len() {
            current.remove(&symbol_stack[symbol_idx]);
        }
        symbol_stack.truncate(symbol_stack_head);
    }
}
