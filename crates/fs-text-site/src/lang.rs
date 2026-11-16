//! A language tag of a site.
//!
//! Pattern: Newtype / Value object, valid on construction (it is also a folder name of the
//! output, so it may hold nothing a path could misuse).

use crate::error::ConfigError;

/// A checked language tag, lower case (`de`, `en`, `pt-br`); also a folder name of the output.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Lang(String);

impl Lang {
    /// Check `s`: 2 or 3 letters, then up to two parts of 2 to 8 letters or digits.
    pub fn parse(s: &str) -> Result<Self, ConfigError> {
        let mut parts = s.split('-');
        let first = parts.next().unwrap_or("");
        let ok_first =
            (2..=3).contains(&first.len()) && first.bytes().all(|b| b.is_ascii_lowercase());
        let rest: Vec<&str> = parts.collect();
        let ok_rest = rest.len() <= 2
            && rest.iter().all(|p| {
                (2..=8).contains(&p.len())
                    && p.bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
            });
        if ok_first && ok_rest {
            Ok(Self(s.to_string()))
        } else {
            Err(ConfigError::BadLanguage(format!(
                "{:?} is not a lower-case language tag like de, en, pt-br",
                crate::host::printable(s)
            )))
        }
    }

    /// The tag.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
