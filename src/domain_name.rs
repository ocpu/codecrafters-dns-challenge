use std::{fmt::Display, hash::Hash, sync::Arc};

use thiserror::Error;

use crate::{
    label::{Label, LabelParseError},
    proto,
};

const MAX_NAME_SIZE: usize = 255;

#[derive(Debug)]
pub enum DomainName {
    Static(usize, &'static str),
    Boxed(Arc<[Label]>),
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum DomainNameIter<'a> {
    Static { cursor: usize, str: &'static str },
    Boxed { index: usize, slice: &'a [Label] },
}

#[derive(Debug, Error)]
pub enum DomainNameParseError {
    #[error("Domain label is too long. Maximum length is 255 got {0}.")]
    NameTooLong(usize),
    #[error(transparent)]
    Label(#[from] LabelParseError),
}

impl DomainName {
    pub fn from_static(str: &'static str) -> DomainName {
        if str.len() > MAX_NAME_SIZE {
            panic!("{}", DomainNameParseError::NameTooLong(str.len()));
        }

        let b = str.as_bytes();
        let mut len = 0;
        let mut cursor = 0;
        let mut last_used = 0;

        while cursor < b.len() {
            if b[cursor] == b'.' {
                Label::valudate_label(&b[last_used..cursor]).unwrap();
                last_used = cursor + 1;
                len += 1;
            }
            cursor += 1;
        }
        if cursor - last_used > 0 {
            Label::valudate_label(&b[last_used..cursor]).unwrap();
            len += 1;
        }

        DomainName::Static(len, str)
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Static(len, _) => *len,
            Self::Boxed(labels) => labels.len(),
        }
    }

    pub fn labels(&self) -> DomainNameIter<'_> {
        match self {
            Self::Static(_, s) => DomainNameIter::Static { cursor: 0, str: s },
            Self::Boxed(labels) => DomainNameIter::Boxed {
                index: 0,
                slice: &labels,
            },
        }
    }

    pub fn equals(&self, other: &proto::DomainName<'_>) -> bool {
        self.len() == other.len()
            && self
                .labels()
                .zip(other.iter())
                .all(|(a, b)| a.eq_ignore_ascii_case(&b))
    }
}

impl core::str::FromStr for DomainName {
    type Err = DomainNameParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() > MAX_NAME_SIZE {
            return Err(DomainNameParseError::NameTooLong(s.len()));
        }
        let b = s.as_bytes();
        let mut labels = Vec::new();
        let mut cursor = 0;
        let mut last_used = 0;

        while cursor < b.len() {
            if b[cursor] == b'.' {
                Label::valudate_label(&b[last_used..cursor])?;
                labels.push(Label::new(&s[last_used..cursor]));
                last_used = cursor + 1;
            }
            cursor += 1;
        }
        if cursor - last_used > 0 {
            Label::valudate_label(&b[last_used..cursor])?;
            labels.push(Label::new(&s[last_used..cursor]));
        }

        Ok(Self::Boxed(Arc::from(labels)))
    }
}

impl<'a> From<&crate::proto::DomainName<'a>> for DomainName {
    fn from(value: &crate::proto::DomainName<'a>) -> Self {
        let list: Vec<_> = value.iter().map(Label::new).collect();
        Self::Boxed(Arc::from(list.into_boxed_slice()))
    }
}

impl Display for DomainName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for item in self.labels() {
            write!(f, "{item}.")?;
        }
        Ok(())
    }
}

impl Clone for DomainName {
    fn clone(&self) -> Self {
        match self {
            Self::Static(len, s) => Self::Static(*len, s),
            Self::Boxed(a) => Self::Boxed(Arc::clone(&a)),
        }
    }
}

impl Hash for DomainName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.labels().for_each(|label| Hash::hash(&label, state));
    }
}

impl PartialEq for DomainName {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.labels().eq(other.labels())
    }
}

impl Eq for DomainName {}

impl<'a> Iterator for DomainNameIter<'a> {
    type Item = Label;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Boxed {
                ref mut index,
                slice,
            } => {
                let Some(res) = slice.get(*index) else {
                    return None;
                };
                *index += 1;
                Some(res.clone())
            }
            Self::Static {
                ref mut cursor,
                ref str,
            } => {
                if *cursor >= str.len() {
                    return None;
                }
                let b = &str.as_bytes();
                let start = *cursor;

                while *cursor < b.len() {
                    if b[*cursor] == b'.' {
                        // SAFETY: Already checked in DomainName::from_static.
                        let res = unsafe { Label::from_static_unchecked(&str[start..*cursor]) };
                        *cursor += 1;
                        return Some(res);
                    }
                    *cursor += 1;
                }
                if b.len() > 0 {
                    // SAFETY: Already checked in DomainName::from_static.
                    Some(unsafe { Label::from_static_unchecked(&str[start..str.len()]) })
                } else {
                    None
                }
            }
        }
    }
}
