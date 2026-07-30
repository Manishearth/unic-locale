//! Unicode Extensions provide a mechanism to extend the `LanguageIdentifier` with
//! additional bits of information.
//!
//! There are four types of extensions:
//!
//!  * Unicode Extensions - marked as `u`.
//!  * Transform Extensions - marked as `t`.
//!  * Private Use Extensions - marked as `x`.
//!  * Other extensions - marked as any alphanumeric character (`a-z`, `0-9`) except for `u`, `t` and `x`.
mod private;
mod transform;
mod unicode;

pub use private::PrivateExtensionList;
pub use transform::TransformExtensionList;
pub use unicode::UnicodeExtensionList;

use std::collections::BTreeMap;
use std::fmt::Write;
use std::iter::Peekable;
use std::str::FromStr;

use tinystr::TinyStr8;

use crate::parser::ParserError;

/// Defines the type of extension.
#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash, PartialOrd, Ord)]
pub enum ExtensionType {
    /// Transform Extension Type marked as `t`.
    Transform,
    /// Unicode Extension Type marked as `u`.
    Unicode,
    /// Private Extension Type marked as `x`.
    Private,
    /// Other Extension Type marked as any alphanumeric character (`a-z`, `0-9`) except for `t`, `u` and `x`.
    Other(char),
}

impl ExtensionType {
    pub fn from_byte(key: u8) -> Result<Self, ParserError> {
        let key = key.to_ascii_lowercase();
        match key {
            b'u' => Ok(ExtensionType::Unicode),
            b't' => Ok(ExtensionType::Transform),
            b'x' => Ok(ExtensionType::Private),
            sign if sign.is_ascii_alphanumeric() => Ok(ExtensionType::Other(char::from(sign))),
            _ => Err(ParserError::InvalidExtension),
        }
    }

    pub(crate) fn from_subtag(subtag: &[u8]) -> Result<Self, ParserError> {
        if subtag.len() != 1 {
            return Err(ParserError::InvalidExtension);
        }
        Self::from_byte(subtag[0])
    }
}

impl std::fmt::Display for ExtensionType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let ch = match self {
            ExtensionType::Unicode => 'u',
            ExtensionType::Transform => 't',
            ExtensionType::Other(n) => *n,
            ExtensionType::Private => 'x',
        };
        f.write_char(ch)
    }
}

/// A map of extensions associated with a given `Locale.
#[derive(Debug, Default, PartialEq, Eq, Clone, Hash, PartialOrd, Ord)]
pub struct ExtensionsMap {
    pub unicode: UnicodeExtensionList,
    pub transform: TransformExtensionList,
    pub other: BTreeMap<char, Vec<TinyStr8>>,
    pub private: PrivateExtensionList,
}

impl ExtensionsMap {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ParserError> {
        let mut iterator = bytes.split(|c| *c == b'-' || *c == b'_').peekable();
        Self::try_from_iter(&mut iterator)
    }

    pub(crate) fn try_from_iter<'a>(
        iter: &mut Peekable<impl Iterator<Item = &'a [u8]>>,
    ) -> Result<Self, ParserError> {
        let mut result = ExtensionsMap::default();

        let mut st = iter.next();
        while let Some(subtag) = st {
            if subtag.is_empty() {
                st = iter.next();
                continue;
            }
            match ExtensionType::from_subtag(subtag)? {
                ExtensionType::Unicode => {
                    result.unicode = UnicodeExtensionList::try_from_iter(iter)?;
                }
                ExtensionType::Transform => {
                    result.transform = TransformExtensionList::try_from_iter(iter)?;
                }
                ExtensionType::Private => {
                    result.private = PrivateExtensionList::try_from_iter(iter)?;
                }
                ExtensionType::Other(ext) => {
                    let mut subtags = vec![];
                    while let Some(next_subtag) = iter.peek() {
                        let slen = next_subtag.len();
                        if (2..=8).contains(&slen)
                            && !next_subtag.iter().any(|c| !c.is_ascii_alphanumeric())
                        {
                            let s = TinyStr8::try_from_utf8(next_subtag)
                                .map_err(|_| ParserError::InvalidSubtag)?;
                            subtags.push(s.to_ascii_lowercase());
                            iter.next();
                        } else {
                            break;
                        }
                    }
                    result.other.entry(ext).or_default().extend(subtags);
                }
            }

            st = iter.next();
        }

        Ok(result)
    }

    pub fn is_empty(&self) -> bool {
        self.unicode.is_empty()
            && self.transform.is_empty()
            && self.private.is_empty()
            && self.other.values().all(Vec::is_empty)
    }
}

impl FromStr for ExtensionsMap {
    type Err = ParserError;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        Self::from_bytes(source.as_bytes())
    }
}

impl std::fmt::Display for ExtensionsMap {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.other.is_empty() {
            write!(f, "{}{}{}", self.transform, self.unicode, self.private)?;
            return Ok(());
        }

        let mut other_sorted = self
            .other
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .collect::<Vec<_>>();
        other_sorted.sort_by_key(|(k, _)| (k.to_ascii_lowercase(), **k));

        // Alphabetic by singleton (0-9, a-z before t)
        for (k, v) in &other_sorted {
            let k_lower = k.to_ascii_lowercase();
            if k_lower < 't' {
                write!(f, "-{}", k_lower)?;
                for subtag in *v {
                    write!(f, "-{}", subtag)?;
                }
            }
        }

        write!(f, "{}", self.transform)?;
        // Defensively format any 't' / 'T' manually inserted into `other`
        for (k, v) in &other_sorted {
            if k.eq_ignore_ascii_case(&'t') {
                write!(f, "-t")?;
                for subtag in *v {
                    write!(f, "-{}", subtag)?;
                }
            }
        }

        write!(f, "{}", self.unicode)?;
        // Defensively format any 'u' / 'U' manually inserted into `other`
        for (k, v) in &other_sorted {
            if k.eq_ignore_ascii_case(&'u') {
                write!(f, "-u")?;
                for subtag in *v {
                    write!(f, "-{}", subtag)?;
                }
            }
        }

        // Alphabetic by singleton (after u, excluding private-use x)
        for (k, v) in &other_sorted {
            let k_lower = k.to_ascii_lowercase();
            if k_lower > 'u' && k_lower != 'x' {
                write!(f, "-{}", k_lower)?;
                for subtag in *v {
                    write!(f, "-{}", subtag)?;
                }
            }
        }

        write!(f, "{}", self.private)?;
        // Defensively format any 'x' / 'X' manually inserted into `other` (private use strictly at the end)
        for (k, v) in &other_sorted {
            if k.eq_ignore_ascii_case(&'x') {
                write!(f, "-x")?;
                for subtag in *v {
                    write!(f, "-{}", subtag)?;
                }
            }
        }

        Ok(())
    }
}
