use core::hash::Hash;
use std::collections::BTreeSet;

use crate::ast::Symbol;
use crate::table::RowId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LeafAssumption {
    relation: Symbol,
    tuple: RowId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DNFAssumption {
    dnf: BTreeSet<BTreeSet<LeafAssumption>>,
}

impl DNFAssumption {
    pub fn zero() -> Self {
        DNFAssumption {
            dnf: BTreeSet::new(),
        }
    }

    pub fn one() -> Self {
        DNFAssumption {
            dnf: BTreeSet::from([BTreeSet::new()]),
        }
    }

    pub fn singleton(leaf: LeafAssumption) -> Self {
        DNFAssumption {
            dnf: BTreeSet::from([BTreeSet::from([leaf])]),
        }
    }

    pub fn conjunct<I>(iter: I) -> Self
    where
        I: Iterator<Item = LeafAssumption>,
    {
        DNFAssumption {
            dnf: BTreeSet::from([iter.collect()]),
        }
    }

    pub fn plus(&self, other: &Self) -> Self {
        let mut new = DNFAssumption {
            dnf: self.dnf.union(&other.dnf).cloned().collect(),
        };
        new.weak_simplify();
        new
    }

    pub fn times(&self, other: &Self) -> Self {
        let mut new = DNFAssumption {
            dnf: BTreeSet::new(),
        };
        for self_conj in &self.dnf {
            for other_conj in &other.dnf {
                new.dnf
                    .insert(self_conj.union(other_conj).cloned().collect());
            }
        }
        new.weak_simplify();
        new
    }

    pub fn weak_simplify(&mut self) {
        let mut to_remove = BTreeSet::new();
        for conj1 in &self.dnf {
            for conj2 in &self.dnf {
                if conj1.is_superset(conj2) && conj1.len() != conj2.len() {
                    to_remove.insert(conj1.clone());
                }
            }
        }
        self.dnf = self.dnf.difference(&to_remove).cloned().collect();
    }

    pub fn quotient(&self, other: &Self) -> Self {
        let mut new = DNFAssumption {
            dnf: BTreeSet::new(),
        };
        for self_conj in &self.dnf {
            for other_conj in &other.dnf {
                new.dnf.insert(self_conj.difference(other_conj).cloned().collect());
            }
        }
        new.weak_simplify();
        new
    }
}
