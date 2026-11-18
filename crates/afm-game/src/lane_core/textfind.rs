//! Searching text without allocating (the ⌘K bar, Cain 2026-11-17: "it just
//! takes absurdly long").
//!
//! The bar looks at every person in the world on every keystroke, and a full
//! world holds over a million of them. `name.to_lowercase().contains(q)`
//! allocates a new string per person and `starts` did it twice more per hit,
//! so one keystroke made millions of small allocations and then sorted every
//! match to show twelve. These helpers compare in place and keep only the
//! best few.

/// Whether `hay` holds `needle` (already lower case), ignoring case.
/// Plain ASCII, nearly every name, is compared in place; anything else
/// (accents, other scripts) takes the full Unicode lower-casing, so the
/// answer is the one `hay.to_lowercase().contains(needle)` gives.
pub fn has_ci(hay: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    if hay.is_ascii() && needle.is_ascii() {
        let (h, n) = (hay.as_bytes(), needle.as_bytes());
        return h.len() >= n.len() && h.windows(n.len()).any(|w| w.eq_ignore_ascii_case(n));
    }
    hay.to_lowercase().contains(needle)
}

/// Whether the whole text or any of its words starts with `needle` (lower
/// case), ignoring case.
pub fn starts_ci(hay: &str, needle: &str) -> bool {
    if hay.is_ascii() && needle.is_ascii() {
        let n = needle.len();
        let starts = |s: &str| s.len() >= n && s.as_bytes()[..n].eq_ignore_ascii_case(needle.as_bytes());
        return starts(hay) || hay.split_whitespace().any(starts);
    }
    let l = hay.to_lowercase();
    l.starts_with(needle) || l.split_whitespace().any(|w| w.starts_with(needle))
}

/// The best `cap` items by score (highest first, the earlier item first on a
/// tie, as a stable sort of everything would give), and how many were
/// offered.
pub struct Best<T> {
    cap: usize,
    items: Vec<(i32, usize, T)>,
    seen: usize,
}

impl<T> Best<T> {
    pub fn new(cap: usize) -> Self {
        Best { cap, items: Vec::with_capacity(cap + 1), seen: 0 }
    }

    pub fn offer(&mut self, score: i32, item: T) {
        let order = self.seen;
        self.seen += 1;
        // Rank: higher score first, then the earlier offer.
        if self.items.len() == self.cap && self.items.last().is_some_and(|l| l.0 >= score) {
            return;
        }
        let at = self.items.partition_point(|x| x.0 >= score);
        self.items.insert(at, (score, order, item));
        self.items.truncate(self.cap);
    }

    /// How many items were offered in all.
    pub fn seen(&self) -> usize {
        self.seen
    }

    pub fn into_items(self) -> Vec<T> {
        self.items.into_iter().map(|x| x.2).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_like_a_lowercased_contains() {
        for (h, n) in [
            ("Marcus Johnson", "ma"),
            ("Marcus Johnson", "son"),
            ("Marcus Johnson", "s j"),
            ("Marcus Johnson", "zz"),
            ("Marcus Johnson", "johnsons"),
            ("Jo", "joe"),
            ("José Álvarez", "josé"),
            ("José Álvarez", "jos"),
            ("José Álvarez", "álv"),
            ("ÉRIC", "éri"),
            ("Texas A&M", "a&m"),
            ("Texas A&M", ""),
        ] {
            assert_eq!(has_ci(h, n), h.to_lowercase().contains(n), "{h:?} has {n:?}");
        }
    }

    #[test]
    fn starts_like_the_old_word_check() {
        for (h, n) in [
            ("Marcus Johnson", "joh"),
            ("Marcus Johnson", "mar"),
            ("Marcus Johnson", "son"),
            ("Marcus  Johnson", "marcus j"),
            ("De'Shawn Lee", "de'"),
            ("Álvaro Núñez", "núñ"),
            ("Álvaro Núñez", "alv"),
        ] {
            let old = h.to_lowercase().split_whitespace().any(|w| w.starts_with(n)) || h.to_lowercase().starts_with(n);
            assert_eq!(starts_ci(h, n), old, "{h:?} starts {n:?}");
        }
    }

    #[test]
    fn best_is_the_head_of_a_stable_sort() {
        let scores = [5, 9, 5, 1, 9, 7, 5, 9, 0, 7, 3, 9, 2, 5];
        for cap in [0, 1, 3, 5, 20] {
            let mut best = Best::new(cap);
            for (k, &s) in scores.iter().enumerate() {
                best.offer(s, k);
            }
            let mut all: Vec<(i32, usize)> = scores.iter().copied().zip(0..).collect();
            all.sort_by_key(|x| std::cmp::Reverse(x.0));
            let want: Vec<usize> = all.iter().take(cap).map(|x| x.1).collect();
            assert_eq!(best.seen(), scores.len());
            assert_eq!(best.into_items(), want, "cap {cap}");
        }
    }
}
