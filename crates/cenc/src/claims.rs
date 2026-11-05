//! The bytes of the file that samples own: a byte justifies one sample at most (§9), so that the
//! table never grows faster than the file, whatever the `trun`s declare.

use std::collections::BTreeMap;

/// The ranges of bytes that samples own: those of the fragments indexed, and those of the
/// fragment being read, which stay owned once it is indexed whole, or are given back.
///
/// A file lays its samples out in order, so a range claimed usually starts past every range
/// before it: such ranges go to a list, sorted as they come, and cost no search. The others,
/// from data offsets pointing back, go to a map, searched in logarithmic time whatever the
/// order an input chooses.
///
/// Invariant: every range is non-empty, no two share a byte, and `furthest` is the largest end.
#[derive(Debug, Default)]
pub(crate) struct Claims {
    in_order: Vec<(usize, usize)>,
    /// From a range's start to its end.
    out_of_order: BTreeMap<usize, usize>,
    furthest: usize,
    /// The length of `in_order` and `furthest` when the fragment being read started.
    kept: (usize, usize),
    /// The starts of the ranges the fragment being read put in `out_of_order`.
    pending_out_of_order: Vec<usize>,
}

impl Claims {
    /// Claims `start..end` for the fragment being read; false, and nothing claimed, when a byte
    /// of it is owned already. An empty range owns nothing and is always granted.
    pub fn claim(&mut self, start: usize, end: usize) -> bool {
        if start >= end {
            return true;
        }
        if start >= self.furthest {
            self.in_order.push((start, end));
            self.furthest = end;
            return true;
        }
        if self.overlaps_in_order(start, end) || self.overlaps_out_of_order(start, end) {
            return false;
        }
        // Starting before `furthest` and sharing no byte with the range that ends there, it
        // ends by `furthest` too, which stays the largest end.
        self.out_of_order.insert(start, end);
        self.pending_out_of_order.push(start);
        true
    }

    /// The fragment being read is indexed: its ranges stay owned.
    pub fn keep(&mut self) {
        self.kept = (self.in_order.len(), self.furthest);
        self.pending_out_of_order.clear();
    }

    /// The fragment being read was not indexed: its ranges are owned no more.
    pub fn give_back(&mut self) {
        (_, self.furthest) = self.kept;
        self.in_order.truncate(self.kept.0);
        for start in self.pending_out_of_order.drain(..) {
            self.out_of_order.remove(&start);
        }
    }

    /// Whether `start..end`, non-empty, shares a byte with a range of `in_order`: being sorted
    /// and disjoint, only the last one starting before `end` can reach into it.
    fn overlaps_in_order(&self, start: usize, end: usize) -> bool {
        let before_end = self.in_order.partition_point(|(from, _)| *from < end);
        before_end
            .checked_sub(1)
            .is_some_and(|last| self.in_order[last].1 > start)
    }

    /// The same for `out_of_order`.
    fn overlaps_out_of_order(&self, start: usize, end: usize) -> bool {
        self.out_of_order
            .range(..end)
            .next_back()
            .is_some_and(|(_, &last_end)| last_end > start)
    }
}

#[cfg(test)]
mod tests {
    use super::Claims;

    #[test]
    fn ranges_that_share_no_byte_are_granted_and_those_that_do_are_refused() {
        let mut claims = Claims::default();
        assert!(claims.claim(10, 20));
        assert!(claims.claim(20, 30), "adjacent, not shared");
        assert!(claims.claim(0, 10));
        assert!(claims.claim(5, 5), "empty");
        assert!(!claims.claim(29, 31));
        assert!(!claims.claim(0, 1));
        assert!(!claims.claim(12, 13));
        assert!(!claims.claim(0, 100));
        assert!(claims.claim(30, 31));
    }

    #[test]
    fn a_fragment_given_back_owns_nothing_and_one_kept_owns_its_ranges() {
        let mut claims = Claims::default();
        assert!(claims.claim(100, 200));
        claims.give_back();
        assert!(claims.claim(100, 200));
        claims.keep();
        assert!(!claims.claim(150, 160), "kept by an indexed fragment");
        assert!(claims.claim(200, 300));
        claims.give_back();
        assert!(claims.claim(50, 100));
        assert!(claims.claim(200, 250));
        claims.keep();
        assert!(!claims.claim(99, 101));
        assert!(!claims.claim(249, 250));
        assert!(claims.claim(250, 251));
        claims.keep();
        assert!(claims.claim(0, 50), "before every range");
        assert!(claims.claim(400, 500));
        claims.give_back();
        assert!(claims.claim(0, 50), "given back");
        assert!(claims.claim(260, 400), "given back too");
    }
}
