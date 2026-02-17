use std::collections::{BTreeMap, BTreeSet};

// Arbitrary type used for ground symbols. For now, should implement Copy.
pub type Symbol = i64;
pub type GroundTuple = Vec<Symbol>;

// Store assumption values per ground tuple. There are two assumption values, an "old" value and a
// "new" value. The old value is the accumulated assumption value for this tuple from prior
// iterations and the new value is the assumption value computed during the current iteration.
pub type Table<A> = BTreeMap<GroundTuple, (A, A)>;

// A "leaf" assumption is the label corresponding to some ground atom in some table. They are created
// due to brackets in rules, and can be combined via plus and times. Arrows "discharge" leaf
// assumptions from assumption semiring values. In representation, this looks identical to a grounded
// atom (a relation name + a ground tuple), and that's not an accident.
pub type LeafAssumption = (String, GroundTuple);

// Interface for assumption interface:
// - Create assumption values from 0, 1, or a leaf assumption.
// - Add or multiply assumption values.
// - Discharge a leaf assumption from an assumption value.
// - Calculate a delta value between two assumptions - given assumption values a and b, delta(a, b)
//   computes some value c such that a + b = a + c.
pub trait Assumption {
    fn is_zero(&self) -> bool;
    fn zero() -> Self;
    fn one() -> Self;
    fn singleton(leaf: LeafAssumption) -> Self;
    fn plus(&self, other: &Self) -> Self;
    fn times(&self, other: &Self) -> Self;
    fn discharge(&self, label: LeafAssumption) -> Self;
    fn delta(&self, other: &Self) -> Self;
}

// NOTE: DNF is not normal w.r.t. simplification modulo the theory of the user-given rules. It is
// normal w.r.t. the structural properties (ACI).
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
    fn is_zero(&self) -> bool {
        self.dnf.is_empty()
    }

    fn zero() -> Self {
        Self {
            dnf: BTreeSet::new(),
        }
    }

    fn one() -> Self {
        Self {
            dnf: BTreeSet::from([BTreeSet::new()]),
        }
    }

    fn singleton(leaf: LeafAssumption) -> Self {
        Self {
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
        let mut new = Self::zero();
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
        let mut new = Self::zero();
        for self_conj in &self.dnf {
            let mut new_conj = self_conj.clone();
            new_conj.remove(&label);
            new.dnf.insert(new_conj);
        }
        new.weak_simplify();
        new
    }

    fn delta(&self, other: &Self) -> Self {
        let mut new = Self::zero();
        for other_conj in &other.dnf {
            if self.dnf.iter().all(|self_conj| !other_conj.is_superset(self_conj)) {
                new.dnf.insert(other_conj.clone());
            }
        }
        new
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dnf_one_zero_singleton() {
        let leaf_a = ("A".to_string(), vec![]);
        let leaf_b = ("B".to_string(), vec![]);
        let leaf_a_prime = ("A".to_string(), vec![]);

        let zero = DNFAssumption::zero();
        let one = DNFAssumption::one();
        let a = DNFAssumption::singleton(leaf_a);
        let b = DNFAssumption::singleton(leaf_b);
        let a_prime = DNFAssumption::singleton(leaf_a_prime);

        assert_ne!(zero, one);
        assert_ne!(zero, a);
        assert_ne!(one, a);
        assert_ne!(a, b);
        assert_eq!(a, a_prime);
    }

    #[test]
    fn dnf_plus_times() {
        let leaf_a = ("A".to_string(), vec![]);
        let leaf_b = ("B".to_string(), vec![]);
        let leaf_c = ("C".to_string(), vec![]);

        let zero = DNFAssumption::zero();
        let one = DNFAssumption::one();
        let a = DNFAssumption::singleton(leaf_a);
        let b = DNFAssumption::singleton(leaf_b);
        let c = DNFAssumption::singleton(leaf_c);

        assert_eq!(zero.plus(&one), one);
        assert_eq!(one.plus(&zero), one);
        assert_eq!(zero.plus(&zero), zero);
        assert_eq!(zero.times(&one), zero);
        assert_eq!(one.times(&zero), zero);
        assert_eq!(one.times(&one), one);

        assert_eq!(zero.plus(&a), a);
        assert_eq!(zero.times(&a), zero);
        assert_eq!(one.times(&a), a);

        assert_eq!(a.plus(&b), b.plus(&a));
        assert_eq!(a.times(&b), b.times(&a));
        assert_eq!(a.plus(&b).plus(&c), a.plus(&(b.plus(&c))));

        assert_eq!(a.plus(&a), a);
        assert_eq!(a.times(&a), a);
        assert_eq!(a.plus(&(a.times(&b))), a);
    }

    #[test]
    fn dnf_discharge() {
        let leaf_a = ("A".to_string(), vec![]);
        let leaf_b = ("B".to_string(), vec![]);

        let a = DNFAssumption::singleton(leaf_a);
        let b = DNFAssumption::singleton(leaf_b.clone());
        let ab = a.times(&b);

        assert_eq!(ab.discharge(leaf_b.clone()), a);
        assert_eq!(a.discharge(leaf_b), a);
    }

    #[test]
    fn dnf_delta() {
        let leaf_a = ("A".to_string(), vec![]);
        let leaf_b = ("B".to_string(), vec![]);

        let zero = DNFAssumption::zero();
        let a = DNFAssumption::singleton(leaf_a);
        let b = DNFAssumption::singleton(leaf_b);
        let ab = a.times(&b);

        assert_eq!(ab.delta(&a), a);
        assert_eq!(a.delta(&ab), zero);
        assert_eq!(ab.delta(&ab), zero);
    }
}
