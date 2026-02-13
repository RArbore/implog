use core::iter::once;
use std::collections::BTreeMap;

use crate::assumption::{Assumption, LeafAssumption};
use crate::ast::{AtomAST, LiteralAST, NameInterner, RuleAST, StatementAST, Symbol, TermAST};
use crate::table::{MapTable, Rows, SetTable, Value};

pub struct Environment<A: Assumption> {
    tables: BTreeMap<Symbol, MapTable>,
    label_makers: BTreeMap<Symbol, SetTable>,
    name_interner: NameInterner,
    assumption_interner: A::Interner,
}

impl<A: Assumption> Environment<A> {
    pub fn new(name_interner: NameInterner) -> Self {
        Environment {
            tables: BTreeMap::new(),
            label_makers: BTreeMap::new(),
            name_interner,
            assumption_interner: A::new_interner(),
        }
    }

    pub fn interpret(&mut self, stmts: &[StatementAST]) {
        let mut rules = vec![];
        for idx in 0..stmts.len() {
            match &stmts[idx] {
                StatementAST::Rule(rule) => {
                    self.register_table_for_atom(&rule.head);
                    for literal in &rule.body {
                        for atom in literal.lhs.iter().chain(once(&literal.rhs)) {
                            self.register_table_for_atom(atom);
                        }
                    }
                    rules.push(rule);
                }
                StatementAST::Question(question) => {
                    for atom in question {
                        self.register_table_for_atom(atom);
                    }
                    self.interpret_rules(&rules);
                    self.interpret_question(question);
                }
            }
        }
    }

    fn register_table_for_atom(&mut self, atom: &AtomAST) {
        let num_determinant = atom.terms.len();
        if let Some(table) = self.tables.get(&atom.relation) {
            assert_eq!(table.num_determinant(), num_determinant);
        } else {
            self.tables
                .insert(atom.relation, MapTable::new(num_determinant));
            self.label_makers
                .insert(atom.relation, SetTable::new(num_determinant));
        }
    }

    fn interpret_rules(&mut self, rules: &Vec<&RuleAST>) {
        for (_, table) in self.tables.iter_mut() {
            table.reset_delta();
        }
        let orders: Vec<_> = rules
            .iter()
            .map(|rule| Self::order_in_literals(&rule.body))
            .collect();
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
        let mut lhs_scratch_row = vec![];
        let mut rhs_scratch_row = vec![];

        'outer: loop {
            let mut answers = vec![];
            for rule_idx in 0..rules.len() {
                let rule = &rules[rule_idx];
                let order = &orders[rule_idx];
                let answer = self.query(
                    &rule
                        .body
                        .iter()
                        .map(|literal| literal.rhs.clone())
                        .collect(),
                    order,
                    true,
                );
                answers.push(answer);
            }

            for (_, table) in self.tables.iter_mut() {
                table.mark_delta();
            }

            for rule_idx in 0..rules.len() {
                let head = &rules[rule_idx].head;
                let speculate = rules[rule_idx].speculate;
                let body = &rules[rule_idx].body;
                let inv_order = &inv_orders[rule_idx];
                let answer = &answers[rule_idx];

                rhs_scratch_row.resize(head.terms.len() + 1, 0);
                for answer_idx in 0..answer.num_rows() {
                    let answer = answer.get_row(answer_idx);
                    Self::substitute_into_atom(&head, answer, &inv_order, &mut rhs_scratch_row);
                    let body_assumption =
                        self.answer_assumption(answer, body, inv_order, &mut lhs_scratch_row);

                    self.insert(
                        head.relation,
                        &mut rhs_scratch_row,
                        body_assumption,
                        speculate,
                    );
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

    fn interpret_question(&mut self, question: &Vec<AtomAST>) {
        let order = Self::order_in_atoms(question);
        let inv_order = order
            .iter()
            .enumerate()
            .map(|(idx, symbol)| (*symbol, idx))
            .collect();
        let answer = self.query(question, &order, false);

        let mut rhs_scratch_row = vec![];
        for answer_idx in 0..answer.num_rows() {
            let answer = answer.get_row(answer_idx);
            for (atom_idx, atom) in question.iter().enumerate() {
                if atom_idx > 0 {
                    print!(", ");
                }
                rhs_scratch_row.resize(atom.terms.len(), 0);
                Self::substitute_into_atom(atom, answer, &inv_order, &mut rhs_scratch_row);
                let assumption = self.tables[&atom.relation].get(&rhs_scratch_row).unwrap().0;
                self.print_atom(assumption.into(), atom.relation, &rhs_scratch_row);
            }
            println!("");
        }
    }

    fn order_in_literals(query: &Vec<LiteralAST>) -> Vec<Symbol> {
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

    fn order_in_atoms(query: &Vec<AtomAST>) -> Vec<Symbol> {
        let mut order = vec![];
        for atom in query {
            for term in &atom.terms {
                if let TermAST::Variable(symbol) = term
                    && !order.contains(symbol)
                {
                    order.push(*symbol);
                }
            }
        }
        order
    }

    fn substitute_into_atom(
        atom: &AtomAST,
        answer: &[Value],
        inv_order: &BTreeMap<Symbol, usize>,
        dst: &mut [Value],
    ) {
        for (term_idx, term) in atom.terms.iter().enumerate() {
            match term {
                TermAST::Variable(symbol) => dst[term_idx] = answer[inv_order[symbol]],
                TermAST::Constant(value) => dst[term_idx] = *value,
            }
        }
    }

    fn answer_assumption(
        &mut self,
        answer: &[Value],
        body: &Vec<LiteralAST>,
        inv_order: &BTreeMap<Symbol, usize>,
        lhs_scratch_row: &mut Vec<Value>,
    ) -> A::Id {
        let mut assumption = A::one(&mut self.assumption_interner);
        assert_eq!(inv_order.len() + body.len(), answer.len());
        for literal_idx in 0..body.len() {
            let literal = &body[literal_idx];
            let mut rhs_assumption = answer[inv_order.len() + literal_idx].into();
            for assumption_idx in 0..literal.lhs.len() {
                let lhs_atom = &literal.lhs[assumption_idx];
                lhs_scratch_row.resize(lhs_atom.terms.len(), 0);
                Self::substitute_into_atom(lhs_atom, answer, &inv_order, lhs_scratch_row);
                if let Some(row_id) = self.label_makers[&lhs_atom.relation].get(&lhs_scratch_row) {
                    let label = LeafAssumption {
                        relation: lhs_atom.relation,
                        tuple: row_id,
                    };
                    rhs_assumption =
                        A::discharge(rhs_assumption, label, &mut self.assumption_interner);
                }
            }
            assumption = A::times(assumption, rhs_assumption, &mut self.assumption_interner);
        }
        assumption
    }

    fn insert(
        &mut self,
        relation: Symbol,
        scratch_row: &mut Vec<Value>,
        body_assumption: A::Id,
        speculate: bool,
    ) {
        let table = self.tables.get_mut(&relation).unwrap();
        let assumption = if speculate {
            let label_maker = self.label_makers.get_mut(&relation).unwrap();
            let row_id = label_maker.insert(&scratch_row[0..table.num_determinant()]);
            let self_assumption = A::singleton(
                LeafAssumption {
                    relation,
                    tuple: row_id,
                },
                &mut self.assumption_interner,
            );
            A::times(
                self_assumption,
                body_assumption,
                &mut self.assumption_interner,
            )
        } else {
            body_assumption
        };

        scratch_row[table.num_determinant()] = assumption.into();
        table.insert(&scratch_row, &mut |a: Value, b: Value| {
            A::plus(a.into(), b.into(), &mut self.assumption_interner).into()
        });
    }

    fn query(&self, query: &Vec<AtomAST>, order: &[Symbol], semi_naive: bool) -> Rows {
        let mut rows = Rows::new(order.len() + query.len());
        if semi_naive && !query.is_empty() {
            let mut shuffled_query = query.clone();
            for semi_naive_idx in 0..query.len() {
                shuffled_query.swap(0, semi_naive_idx);
                self.query_helper(
                    &shuffled_query,
                    order,
                    &mut rows,
                    &BTreeMap::new(),
                    &mut vec![],
                    true,
                    semi_naive_idx,
                );
                shuffled_query.swap(0, semi_naive_idx);
            }
        } else {
            self.query_helper(
                query,
                order,
                &mut rows,
                &BTreeMap::new(),
                &mut vec![],
                false,
                0,
            );
        }
        rows
    }

    fn query_helper(
        &self,
        query: &[AtomAST],
        order: &[Symbol],
        rows: &mut Rows,
        assignment: &BTreeMap<Symbol, Value>,
        assumptions: &mut Vec<Value>,
        first: bool,
        semi_naive_shuffle: usize,
    ) {
        if query.is_empty() {
            let row_id = rows.alloc_row();
            let row = rows.get_row_mut(row_id);
            for (idx, var) in order.into_iter().enumerate() {
                row[idx] = assignment[var];
            }
            if !assumptions.is_empty() {
                row[order.len() + semi_naive_shuffle] = assumptions[0];
                for idx in 1..assumptions.len() {
                    if idx <= semi_naive_shuffle {
                        row[order.len() + idx - 1] = assumptions[idx];
                    } else {
                        row[order.len() + idx] = assumptions[idx];
                    }
                }
            }
            return;
        }

        let atom = &query[0];
        let rest = &query[1..];
        let rhs_table = &self.tables[&atom.relation];
        assert_eq!(rhs_table.num_determinant(), atom.terms.len());

        'outer: for (row, _) in rhs_table.rows(first) {
            let mut new_assignment = assignment.clone();
            for col_idx in 0..rhs_table.num_determinant() {
                let in_row = row[col_idx];
                match atom.terms[col_idx] {
                    TermAST::Variable(var) => {
                        if let Some(old) = new_assignment.insert(var, in_row)
                            && old != in_row
                        {
                            continue 'outer;
                        }
                    }
                    TermAST::Constant(value) => {
                        if value != in_row {
                            continue 'outer;
                        }
                    }
                }
            }

            assumptions.push(*row.last().unwrap());
            self.query_helper(
                rest,
                order,
                rows,
                &new_assignment,
                assumptions,
                false,
                semi_naive_shuffle,
            );
            assumptions.pop();
        }
    }

    fn print_atom(&self, assumption: A::Id, relation: Symbol, tuple: &[Value]) {
        A::print(assumption, &self.assumption_interner, |leaf| {
            print!("{}(", self.name_interner.resolve(leaf.relation).unwrap());
            let tuple = self.tables[&leaf.relation].index(leaf.tuple);
            for idx in 0..tuple.len() - 1 {
                if idx > 0 {
                    print!(", ");
                }
                print!("{}", tuple[idx]);
            }
            print!(")");
        });

        print!(" : {}(", self.name_interner.resolve(relation).unwrap());
        for idx in 0..tuple.len() {
            if idx > 0 {
                print!(", ");
            }
            print!("{}", tuple[idx]);
        }
        print!(")")
    }
}
