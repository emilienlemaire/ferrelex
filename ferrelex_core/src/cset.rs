use std::hash::Hash;

#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct CSet(Vec<(isize, isize)>);

impl CSet {
    pub fn max_code() -> isize {
        0x10ffff
    }

    pub fn min_code() -> isize {
        -1
    }

    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn singleton(i: isize) -> Self {
        Self(vec![(i, i)])
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn interval(i: isize, j: isize) -> Self {
        if i < j {
            Self(vec![(i, j)])
        } else {
            Self(vec![(j, i)])
        }
    }

    pub fn any() -> Self {
        Self::interval(0, Self::max_code())
    }

    pub fn eof() -> Self {
        Self::singleton(-1)
    }

    pub fn union(&self, o: &Self) -> Self {
        let mut all: Vec<(isize, isize)> = Vec::with_capacity(self.0.len() + o.0.len());
        all.extend(&self.0);
        all.extend(&o.0);

        all.sort_by_key(|&(a, _)| a);

        let mut merged: Vec<(isize, isize)> = Vec::new();
        for (a, b) in all {
            if let Some(&mut (_, ref mut last_end)) = merged.last_mut() {
                if *last_end + 1 >= a {
                    *last_end = (*last_end).max(b);
                    continue;
                }
            }
            merged.push((a, b));
        }

        CSet(merged)
    }

    pub fn union_vec(mut l: Vec<Self>) -> Self {
        let mut v: Vec<(isize, isize)> = l.iter_mut().flat_map(|c| c.0.clone()).collect();
        v.sort_by_key(|&(a, _)| a);
        v.iter().fold(Self::new(), |acc, (i, j)| {
            Self::interval(*i, *j).union(&acc)
        })
    }

    pub fn complement(&self) -> Self {
        let mut out = Vec::new();
        let mut start = -1;
        let mut idx = 0;

        if let Some((-1, j)) = self.0.first() {
            start = j + 1;
            idx = 1;
        }

        for (i, j) in &self.0[idx..] {
            out.push((start, i - 1));
            start = j + 1;
        }

        if start <= Self::max_code() {
            out.push((start, Self::max_code()));
        }
        CSet(out)
    }

    pub fn intersection(&self, o: &Self) -> Self {
        self.complement().union(&o.complement()).complement()
    }

    pub fn difference(&self, o: &Self) -> Self {
        self.complement().union(o).complement()
    }
}

impl TryFrom<Vec<(isize, isize)>> for CSet {
    type Error = String;

    fn try_from(value: Vec<(isize, isize)>) -> Result<Self, Self::Error> {
        let mut prev = -1;
        for (a, b) in &value {
            if (*a as i64) < prev {
                return Err(format!(
                    "Vec not in an increasing order: found [_, (_, {prev}), ({a}, {b})]"
                ));
            }
            if (*a as i64) == prev {
                return Err(format!(
                    "Found an adjacent range: [_, (_, {prev}), ({a}, {b})]"
                ));
            }
            if a > b {
                return Err(format!("Found an malformed range: [_, ({a}, {b})]"));
            }
            prev = *b as i64;
        }
        Ok(CSet(value))
    }
}

impl TryFrom<&[(isize, isize)]> for CSet {
    type Error = String;

    /// NOTE: This will allocate a vector if the ranges are valid.
    fn try_from(value: &[(isize, isize)]) -> Result<Self, Self::Error> {
        let mut prev = -1;
        for (a, b) in value {
            if (*a as i64) < prev {
                return Err(format!(
                    "Vec not in an increasing order: found [_, (_, {prev}), ({a}, {b})]"
                ));
            }
            if (*a as i64) == prev {
                return Err(format!(
                    "Found an adjacent range: [_, (_, {prev}), ({a}, {b})]"
                ));
            }
            if a > b {
                return Err(format!("Found an malformed range: [_, ({a}, {b})]"));
            }
            prev = *b as i64;
        }
        Ok(CSet(value.to_vec()))
    }
}

#[allow(clippy::from_over_into)] // I don't want to impl `From<CSet>` for `Vec<(isize, isize)>`.
impl Into<Vec<(isize, isize)>> for CSet {
    fn into(self) -> Vec<(isize, isize)> {
        self.0
    }
}

impl Hash for CSet {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union() {
        let c1 = CSet::interval(0, 5);
        let c2 = CSet::interval(3, 10);
        assert_eq!(c1.union(&c2), CSet::interval(0, 10));
        let c1 = CSet::interval(0, 5);
        let c2 = CSet::interval(3, 10);
        assert_eq!(c2.union(&c1), CSet::interval(0, 10));
        let c1 = CSet::interval(0, 11);
        let c2 = CSet::interval(3, 10);
        assert_eq!(c1.union(&c2), CSet::interval(0, 11));
    }
}
