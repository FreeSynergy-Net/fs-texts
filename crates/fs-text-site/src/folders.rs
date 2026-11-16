//! Folders that no page stands for: they get an overview page of their own.
//!
//! Pattern: Specification over the set of addresses (a folder is every proper prefix of a page
//! address that is not a page itself). The overview lists the pages below, with the titles
//! from their heads; it has no text of its own beyond the label that says it was generated.

use std::collections::BTreeSet;

/// The folders (address prefixes, never the root) that are a prefix of a page but no page.
#[must_use]
pub fn missing(paths: &BTreeSet<Vec<String>>) -> BTreeSet<Vec<String>> {
    let mut out = BTreeSet::new();
    for p in paths {
        for n in 1..p.len() {
            let f = &p[..n];
            if !paths.contains(f) {
                out.insert(f.to_vec());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::missing;
    use std::collections::BTreeSet;

    fn set(v: &[&[&str]]) -> BTreeSet<Vec<String>> {
        v.iter()
            .map(|p| p.iter().map(|s| (*s).to_string()).collect())
            .collect()
    }

    #[test]
    fn a_folder_without_page_is_found_and_a_folder_with_one_is_not() {
        let m = missing(&set(&[&[], &["a", "b", "c"], &["x"], &["x", "y"]]));
        assert_eq!(m, set(&[&["a"], &["a", "b"]]));
    }
}
