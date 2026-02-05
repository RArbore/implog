use core::hash::Hash;
use std::collections::BTreeSet;

use crate::ast::Symbol;
use crate::table::RowId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LeafAssumption {
    pub relation: Symbol,
    pub tuple: RowId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DNFAssumption {
    pub dnf: BTreeSet<BTreeSet<LeafAssumption>>,
}

impl DNFAssumption {
    pub fn singleton(leaf: LeafAssumption) -> Self {
        DNFAssumption {
            dnf: BTreeSet::from([BTreeSet::from([leaf])]),
        }
    }

    pub fn plus(&self, other: &Self) -> Self {
        
    }
}
