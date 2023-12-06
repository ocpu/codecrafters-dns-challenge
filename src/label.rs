use std::{fmt::Display, hash::Hash, sync::Arc};

use thiserror::Error;

const MAX_LABEL_SIZE: usize = 63;

#[derive(Debug)]
pub enum Label {
    Boxed(Arc<str>),
    Static(&'static str),
}

#[derive(Debug, Error)]
pub enum LabelParseError {
    #[error("Domain label is too long. Maximum length is 63 got {0}.")]
    LabelTooLong(usize),
    #[error(
        "Domain label includes an illegal character at position {position}. {char} ({char:x?})"
    )]
    IllegalLabelChar { char: u8, position: usize },
}

impl Label {
    pub fn new(label: &str) -> Self {
        Self::Boxed(Arc::from(label))
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Boxed(l) => l.len(),
            Self::Static(l) => l.len(),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Boxed(l) => l.as_bytes(),
            Self::Static(l) => l.as_bytes(),
        }
    }

    pub const unsafe fn from_static_unchecked(s: &'static str) -> Self {
        Self::Static(s)
    }

    pub const fn valudate_label(label_bytes: &[u8]) -> Result<(), LabelParseError> {
        use LabelParseError::*;

        if label_bytes.len() > MAX_LABEL_SIZE {
            return Err(LabelTooLong(label_bytes.len()));
        }

        let mut i = 0;
        while i < label_bytes.len() {
            match &label_bytes[i] {
                b'A'..=b'Z' | b'a'..=b'z' => {}
                b'0'..=b'9' if i != 0 => {}
                b'-' if i != 0 && i + 1 != label_bytes.len() => {}
                c => {
                    return Err(IllegalLabelChar {
                        char: *c,
                        position: i,
                    })
                }
            }
            i += 1;
        }

        Ok(())
    }
}

impl AsRef<str> for Label {
    fn as_ref(&self) -> &str {
        match self {
            Self::Boxed(l) => l.as_ref(),
            Self::Static(l) => l.as_ref(),
        }
    }
}

impl core::ops::Deref for Label {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl Hash for Label {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for c in self.as_bytes() {
            state.write_u8(match c {
                b'A'..=b'Z' | b'a'..=b'z' => c & 0b01011111,
                c => *c,
            });
        }
    }
}

impl PartialEq for Label {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref().eq_ignore_ascii_case(other.as_ref())
    }
}

impl Eq for Label {}

impl Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl Clone for Label {
    fn clone(&self) -> Self {
        match self {
            Self::Static(s) => Self::Static(s),
            Self::Boxed(a) => Self::Boxed(Arc::clone(&a)),
        }
    }
}
