use core::hash::Hasher;
use std::collections::BTreeSet;
use std::collections::btree_set::Iter;
use std::iter::Peekable;

use hashbrown::HashTable;
use hashbrown::hash_table::Entry;
use rustc_hash::FxHasher;

pub type Value = u32;
pub type RowId = u64;
type HashCode = u64;

#[derive(Debug)]
struct TableEntry {
    hash: HashCode,
    row: RowId,
}

#[derive(Debug)]
pub struct Rows {
    buffer: Vec<Value>,
    num_columns: usize,
}

#[derive(Debug)]
pub struct Table {
    rows: Rows,
    table: HashTable<TableEntry>,
    deleted_rows: BTreeSet<RowId>,
    delta: RowId,
}

#[derive(Debug)]
struct TableRows<'a> {
    table: &'a Table,
    row: RowId,
    deleted_iter: Peekable<Iter<'a, RowId>>,
}

fn hash(determinant: &[Value]) -> HashCode {
    let mut hasher = FxHasher::default();
    for val in determinant {
        hasher.write_u32(*val);
    }
    hasher.finish()
}

impl Rows {
    pub fn new(num_columns: usize) -> Self {
        Rows {
            buffer: vec![],
            num_columns,
        }
    }

    pub fn num_rows(&self) -> RowId {
        (self.buffer.len() / self.num_columns) as RowId
    }

    pub fn num_columns(&self) -> usize {
        self.num_columns
    }

    pub fn get_row(&self, row: RowId) -> &[Value] {
        let start = (row as usize) * self.num_columns;
        &self.buffer[start..start + self.num_columns]
    }

    pub fn get_row_mut(&mut self, row: RowId) -> &mut [Value] {
        let start = (row as usize) * self.num_columns;
        &mut self.buffer[start..start + self.num_columns]
    }

    pub fn add_row(&mut self, row: &[Value]) -> RowId {
        assert_eq!(row.len(), self.num_columns);
        let row_id = self.num_rows();
        self.buffer.extend(row);
        row_id
    }

    pub fn alloc_row(&mut self) -> RowId {
        let row_id = self.num_rows();
        self.buffer.resize(self.buffer.len() + self.num_columns, 0);
        row_id
    }
}

impl Table {
    pub fn new(num_determinant: usize) -> Self {
        Self {
            rows: Rows {
                buffer: vec![],
                num_columns: num_determinant + 1,
            },
            table: HashTable::new(),
            deleted_rows: BTreeSet::new(),
            delta: 0,
        }
    }

    pub fn num_determinant(&self) -> usize {
        self.rows.num_columns - 1
    }

    pub fn reset_delta(&mut self) {
        self.delta = 0;
    }

    pub fn mark_delta(&mut self) {
        self.delta = self.rows.num_rows();
    }

    pub fn changed(&self) -> bool {
        self.delta != self.rows.num_rows()
    }

    pub fn insert<'a, M>(&'a mut self, row: &[Value], merge: &mut M) -> (&'a [Value], RowId)
    where
        M: FnMut(Value, Value) -> Value,
    {
        let num_determinant = self.num_determinant();
        assert_eq!(row.len(), num_determinant + 1);
        let determinant = &row[0..num_determinant];
        let hash = hash(determinant);
        let entry = self.table.entry(
            hash,
            |te| te.hash == hash && &self.rows.get_row(te.row)[0..num_determinant] == determinant,
            |te| te.hash,
        );
        let vacant = match entry {
            Entry::Occupied(occupied) => {
                let row_id = occupied.get().row;
                let old = self.rows.get_row(row_id)[num_determinant];
                let new = row[num_determinant];
                let merged = merge(old, new);
                if merged == old {
                    return (self.rows.get_row(row_id), row_id);
                }
                let (_, vacant) = occupied.remove();
                self.deleted_rows.insert(row_id);
                vacant
            }
            Entry::Vacant(vacant) => vacant,
        };
        let row_id = self.rows.add_row(row);
        vacant.insert(TableEntry { hash, row: row_id });
        (self.rows.get_row(row_id), row_id)
    }

    pub fn get(&self, determinant: &[Value]) -> Option<Value> {
        let num_determinant = self.num_determinant();
        assert_eq!(determinant.len(), num_determinant);
        let hash = hash(determinant);
        let te = self.table.find(hash, |te| {
            te.hash == hash && &self.rows.get_row(te.row)[0..num_determinant] == determinant
        });
        te.map(|te| self.rows.get_row(te.row)[num_determinant])
    }

    pub fn delete(&mut self, row_id: RowId) -> &[Value] {
        let row = self.rows.get_row(row_id);
        let determinant = &row[0..self.num_determinant()];
        let hash = hash(determinant);
        let entry = self
            .table
            .entry(hash, |te| te.hash == hash && te.row == row_id, |te| te.hash);
        let Entry::Occupied(occupied) = entry else {
            panic!();
        };
        occupied.remove();
        self.deleted_rows.insert(row_id);
        row
    }

    pub fn rows(&self, after_delta: bool) -> impl Iterator<Item = (&[Value], RowId)> + '_ {
        TableRows {
            table: self,
            row: if after_delta { self.delta } else { 0 },
            deleted_iter: self.deleted_rows.iter().peekable(),
        }
    }

    pub fn split_rows(
        &self,
        after_delta: bool,
    ) -> impl Iterator<Item = (&[Value], Value, RowId)> + '_ {
        self.rows(after_delta)
            .map(|(row, id)| (&row[0..row.len() - 1], row[row.len() - 1], id))
    }

    pub fn num_rows(&self) -> RowId {
        self.rows.num_rows() - self.deleted_rows.len() as RowId
    }
}

impl<'a> Iterator for TableRows<'a> {
    type Item = (&'a [Value], RowId);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(recent_deleted) = self.deleted_iter.peek() {
            if **recent_deleted > self.row {
                break;
            } else if **recent_deleted == self.row {
                self.row += 1;
            }
            self.deleted_iter.next();
        }

        if self.row >= self.table.rows.num_rows() {
            None
        } else {
            let row = self.row;
            self.row += 1;
            Some((self.table.rows.get_row(row), row))
        }
    }
}
