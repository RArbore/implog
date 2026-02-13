use core::hash::Hash;
use core::mem::drop;
use std::collections::BTreeSet;

use crate::ast::Symbol;
use crate::interner::{InternId, Interner};
use crate::table::{RowId, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LeafAssumption {
    pub relation: Symbol,
    pub tuple: RowId,
}

pub trait Assumption {
    type Interner;
    type Id: From<Value> + Into<Value>;

    fn new_interner() -> Self::Interner;

    fn one(interner: &mut Self::Interner) -> Self::Id;
    fn singleton(leaf: LeafAssumption, interner: &mut Self::Interner) -> Self::Id;

    fn plus(a: Self::Id, b: Self::Id, interner: &mut Self::Interner) -> Self::Id;
    fn times(a: Self::Id, b: Self::Id, interner: &mut Self::Interner) -> Self::Id;
    fn discharge(a: Self::Id, label: LeafAssumption, interner: &mut Self::Interner) -> Self::Id;

    fn print<F>(a: Self::Id, interner: &Self::Interner, print_leaf: F)
    where
        F: Fn(LeafAssumption);
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DNFAssumption {
    pub dnf: BTreeSet<BTreeSet<LeafAssumption>>,
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

    pub fn discharge(&self, label: LeafAssumption) -> Self {
        let mut new = DNFAssumption {
            dnf: BTreeSet::new(),
        };
        for self_conj in &self.dnf {
            let mut new_conj = self_conj.clone();
            new_conj.remove(&label);
            new.dnf.insert(new_conj);
        }
        new.weak_simplify();
        new
    }
}

impl Assumption for DNFAssumption {
    type Interner = Interner<DNFAssumption>;
    type Id = InternId<DNFAssumption>;

    fn new_interner() -> Self::Interner {
        Self::Interner::new()
    }

    fn one(interner: &mut Self::Interner) -> Self::Id {
        interner.intern(Self::one())
    }

    fn singleton(leaf: LeafAssumption, interner: &mut Self::Interner) -> Self::Id {
        interner.intern(Self::singleton(leaf))
    }

    fn plus(a: Self::Id, b: Self::Id, interner: &mut Self::Interner) -> Self::Id {
        let a = interner.get(a);
        let b = interner.get(b);
        let c = a.plus(&b);
        drop(a);
        drop(b);
        interner.intern(c)
    }

    fn times(a: Self::Id, b: Self::Id, interner: &mut Self::Interner) -> Self::Id {
        let a = interner.get(a);
        let b = interner.get(b);
        let c = a.times(&b);
        drop(a);
        drop(b);
        interner.intern(c)
    }

    fn discharge(a: Self::Id, label: LeafAssumption, interner: &mut Self::Interner) -> Self::Id {
        let a = interner.get(a);
        let b = a.discharge(label);
        drop(a);
        interner.intern(b)
    }

    fn print<F>(a: Self::Id, interner: &Self::Interner, print_leaf: F)
    where
        F: Fn(LeafAssumption),
    {
        let a = interner.get(a);
        if a.dnf.is_empty() {
            print!("False");
        }
        for (conj_idx, conj) in a.dnf.iter().enumerate() {
            if conj_idx > 0 {
                print!(" + ");
            }
            if conj.is_empty() {
                print!("True");
            }
            for (leaf_idx, leaf) in conj.iter().enumerate() {
                if leaf_idx > 0 {
                    print!(" * ");
                }
                print_leaf(*leaf);
            }
        }
    }
}
