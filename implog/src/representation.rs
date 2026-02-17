use std::collections::{BTreeMap, BTreeSet};

// Arbitrary type used for ground symbols. For now, should implement Copy.
pub type Symbol = i64;
pub type GroundTuple = Vec<Symbol>;

// TODO: use (old, new) for assumption value.
pub type Table<A> = BTreeMap<GroundTuple, A>;

// A "leaf" assumption is the label corresponding to some ground atom in some table. They are created
// due to brackets in rules, and can be combined via plus and times. Arrows "discharge" leaf
// assumptions from assumption semiring values. In representation, this looks identical to a grounded
// atom (a relation name + a ground tuple), and that's not an accident.
pub type LeafAssumption = (String, GroundTuple);

// Interface for assumption interface - create assumption values from 0, 1, or leaf assumptions,
// combine them with plus and times, and discharge leaf assumptions from them.
pub trait Assumption {
    fn zero() -> Self;
    fn one() -> Self;
    fn singleton(leaf: LeafAssumption) -> Self;
    fn plus(&self, other: &Self) -> Self;
    fn times(&self, other: &Self) -> Self;
    fn discharge(&self, label: LeafAssumption) -> Self;
}

// NOTE: DNF is not normal w.r.t. simplification modulo the theory of the user-given rules. It is
// enforced to be normal w.r.t. the structural properties (i.e. ACI) via weak_simplify().
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DNFAssumption {
    pub dnf: BTreeSet<BTreeSet<LeafAssumption>>,
}

impl DNFAssumption {
    fn weak_simplify(&mut self) {
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
}

impl Assumption for DNFAssumption {
    fn zero() -> Self {
        DNFAssumption {
            dnf: BTreeSet::new(),
        }
    }

    fn one() -> Self {
        DNFAssumption {
            dnf: BTreeSet::from([BTreeSet::new()]),
        }
    }

    fn singleton(leaf: LeafAssumption) -> Self {
        DNFAssumption {
            dnf: BTreeSet::from([BTreeSet::from([leaf])]),
        }
    }

    fn plus(&self, other: &Self) -> Self {
        let mut new = DNFAssumption {
            dnf: self.dnf.union(&other.dnf).cloned().collect(),
        };
        new.weak_simplify();
        new
    }

    fn times(&self, other: &Self) -> Self {
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

    fn discharge(&self, label: LeafAssumption) -> Self {
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
