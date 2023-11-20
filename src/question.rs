use std::fmt::Display;

use crate::{
    domain_name::{DomainName, DomainNameParseError},
    types::{QClass, QType, UnknownClass, UnknownType},
};

pub struct Question<'a> {
    name: DomainName<'a>,
    q_type: QType,
    q_class: QClass,
}

#[derive(Debug)]
pub enum QuestionParseError {
    DomainName(DomainNameParseError),
    UnknownQType(u16),
    UnknownQClass(u16),
    EOF,
}

impl From<DomainNameParseError> for QuestionParseError {
    fn from(value: DomainNameParseError) -> Self {
        Self::DomainName(value)
    }
}

impl From<UnknownType> for QuestionParseError {
    fn from(value: UnknownType) -> Self {
        Self::UnknownQType(value.0)
    }
}

impl From<UnknownClass> for QuestionParseError {
    fn from(value: UnknownClass) -> Self {
        Self::UnknownQClass(value.0)
    }
}

impl<'a> Question<'a> {
    pub fn new(q_type: QType, q_class: QClass, name: DomainName<'a>) -> Self {
        Self {
            name,
            q_type,
            q_class,
        }
    }

    pub fn name(&self) -> &DomainName<'a> {
        &self.name
    }

    pub fn q_type(&self) -> &QType {
        &self.q_type
    }

    pub fn q_class(&self) -> &QClass {
        &self.q_class
    }

    pub fn len_in_packet(&self) -> usize {
        4 + self.name.len_in_packet()
    }

    pub fn try_parse(
        buffer: &'a [u8],
        offset: usize,
    ) -> Result<Option<(Self, usize)>, QuestionParseError> {
        let Some((name, name_size)) = DomainName::try_parse(&buffer, offset)? else {
            return Ok(None);
        };
        if offset + name_size + 4 > buffer.len() {
            return Err(QuestionParseError::EOF);
        }
        let q_type = QType::try_from(u16::from_be_bytes([
            buffer[offset + name_size],
            buffer[offset + name_size + 1],
        ]))?;
        let q_class = QClass::try_from(u16::from_be_bytes([
            buffer[offset + name_size + 2],
            buffer[offset + name_size + 3],
        ]))?;
        Ok(Some((
            Self {
                name,
                q_type,
                q_class,
            },
            name_size + 4,
        )))
    }
}

impl<'a> Display for Question<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {:?} {:?}", self.name, self.q_class, self.q_type)
    }
}
