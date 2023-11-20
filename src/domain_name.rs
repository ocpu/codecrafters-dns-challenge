use std::{fmt::Display, hash::Hash, rc::Rc};

use crate::label::{Label, LabelParseError, LabelParseResult};

const MAX_NAME_SIZE: usize = 255;

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct DomainName<'a>(Rc<[Label<'a>]>);

#[derive(Debug)]
pub enum DomainNameParseError {
    NameTooLarge,
    Label(LabelParseError),
    CyclicPointers,
}

impl From<LabelParseError> for DomainNameParseError {
    fn from(value: LabelParseError) -> Self {
        Self::Label(value)
    }
}

impl<'a> DomainName<'a> {
    pub fn new(labels: Rc<[Label<'a>]>) -> Self {
        Self(labels)
    }

    pub fn from_static(str: &'static str) -> DomainName<'static> {
        DomainName::from_str(str)
            .expect("Failed to parse input string as domain name")
            .expect("Input string was empty")
    }

    pub fn from_str(s: &'a str) -> Result<Option<Self>, DomainNameParseError> {
        let bytes = s.as_bytes();
        if bytes.len() > MAX_NAME_SIZE {
            return Err(DomainNameParseError::NameTooLarge);
        }
        if bytes.len() == 0 {
            return Ok(None);
        }
        let mut labels = Vec::new();
        let mut len = 0;
        let mut str = s;
        loop {
            match Label::from_str(&str) {
                Err(e) => return Err(DomainNameParseError::Label(e)),
                Ok(LabelParseResult::End) => break,
                Ok(LabelParseResult::Pointer(pointer)) => {
                    return Err(DomainNameParseError::Label(
                        LabelParseError::IllegalLabelChar((pointer as u8) & 0xc0),
                    ))
                }
                Ok(LabelParseResult::Label(label)) => {
                    let label_len = label.len();
                    labels.push(label);
                    if len != 0 {
                        len += 1;
                    }
                    len += label_len;
                    if len > MAX_NAME_SIZE {
                        return Err(DomainNameParseError::NameTooLarge);
                    }
                    if let Some(&c) = &str.as_bytes().get(label_len) {
                        if c != b'.' {
                            return Err(DomainNameParseError::Label(
                                LabelParseError::IllegalLabelChar(c),
                            ));
                        } else {
                            str = &str[label_len + 1..];
                        }
                    } else {
                        break;
                    }
                }
            }
        }
        Ok(Some(Self(Rc::from(labels))))
    }

    pub fn try_parse(
        buffer: &'a [u8],
        offset: usize,
    ) -> Result<Option<(Self, usize)>, DomainNameParseError> {
        use DomainNameParseError::*;

        if buffer.len() - offset == 0 {
            return Ok(None);
        }
        let mut cursor = offset;
        let mut len = 0;
        let mut size = 0;
        let mut labels = Vec::new();
        let mut seen_pointers = Vec::new();
        loop {
            match self::Label::try_parse(buffer, cursor)? {
                (label_size, LabelParseResult::Label(label)) => {
                    cursor += label_size;
                    let label_len = label.len();
                    labels.push(label);
                    if seen_pointers.is_empty() {
                        size += label_size;
                    }
                    if len != 0 {
                        len += 1;
                    }
                    len += label_len;
                    if len > MAX_NAME_SIZE {
                        return Err(NameTooLarge);
                    }
                }
                (pointer_size, LabelParseResult::Pointer(pointer)) => {
                    cursor = pointer;
                    if seen_pointers.is_empty() {
                        size += pointer_size;
                    }
                    if seen_pointers.contains(&pointer) {
                        return Err(CyclicPointers);
                    } else {
                        seen_pointers.push(pointer);
                    }
                }
                (end_size, LabelParseResult::End) => {
                    if seen_pointers.is_empty() {
                        size += end_size;
                    }
                    break;
                }
            }
        }
        Ok(Some((Self(Rc::from(labels)), size)))
    }
}

impl<'a> DomainName<'a> {
    pub fn len_in_packet(&self) -> usize {
        1 + self.0.iter().map(|part| part.len() + 1).sum::<usize>()
    }

    pub fn labels(&self) -> &[Label<'a>] {
        &self.0
    }
}

impl<'a> Display for DomainName<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for item in self.0.as_ref() {
            write!(f, "{item}.")?;
        }
        Ok(())
    }
}

impl<'a> Clone for DomainName<'a> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}
