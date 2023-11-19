use std::fmt::Display;

use crate::types::{CowDomainName, DomainName, DomainNameParseError, QClass, QType};

pub struct Question<'a> {
    name: CowDomainName<'a>,
    q_type: QType,
    q_class: QClass,
}

#[derive(Debug)]
pub enum QuestionParseError {
    DomainName(DomainNameParseError),
    UnknownQType,
    UnkonwnQClass,
    EOF,
}

impl<'a> Question<'a> {
    pub fn new(q_type: QType, q_class: QClass, name: CowDomainName<'a>) -> Self {
        Self {
            name,
            q_type,
            q_class,
        }
    }

    pub fn name(&self) -> &CowDomainName<'a> {
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

    pub fn try_parse(buffer: &'a [u8]) -> Result<(Self, usize), QuestionParseError> {
        let name: DomainName<'a> = buffer
            .try_into()
            .map_err(|e| QuestionParseError::DomainName(e))?;
        let size = name.len_in_packet();
        if size + 4 > buffer.len() {
            return Err(QuestionParseError::EOF);
        }
        let q_type = QType::try_from(u16::from_be_bytes([buffer[size], buffer[size + 1]]))
            .map_err(|_| QuestionParseError::UnknownQType)?;
        let q_class = QClass::try_from(u16::from_be_bytes([buffer[size + 2], buffer[size + 3]]))
            .map_err(|_| QuestionParseError::UnkonwnQClass)?;
        Ok((
            Self {
                name: name.into(),
                q_type,
                q_class,
            },
            size + 4,
        ))
    }
}

impl<'a> Display for Question<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {:?} {:?}", self.name, self.q_class, self.q_type)
    }
}
