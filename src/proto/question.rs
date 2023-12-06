use std::fmt;

use thiserror::Error;

use super::{
    class::QClass, domain_name::DomainName, label::LabelError, types::QType, FromPacketBytes,
};

#[derive(Clone, Copy)]
pub struct Question<'data> {
    pub(super) offset: usize,
    pub(super) buffer: &'data [u8],
}

#[derive(Debug, Error)]
pub enum QuestionError {
    #[error(transparent)]
    Label(#[from] LabelError),
    #[error("TODO")]
    EOF,
}

impl<'data> Question<'data> {
    pub fn name(&self) -> DomainName<'data> {
        DomainName::parse(self.buffer, self.offset)
            .expect("Domain name to be checked before an instance of Question was created")
            .expect("Domain name to be present")
    }

    pub fn q_type(&self) -> QType {
        let name_size = self.name().size_in_packet();
        QType::try_from(u16::from_be_bytes([
            *self
                .buffer
                .get(self.offset + name_size)
                .expect("Q type value to be present"),
            *self
                .buffer
                .get(self.offset + name_size + 1)
                .expect("Q type value to be present"),
        ]))
        .expect("Q type to be valid")
    }

    pub fn q_class(&self) -> QClass {
        let name_size = self.name().size_in_packet();
        QClass::try_from(u16::from_be_bytes([
            *self
                .buffer
                .get(self.offset + name_size + 2)
                .expect("Q class value to be present"),
            *self
                .buffer
                .get(self.offset + name_size + 3)
                .expect("Q class value to be present"),
        ]))
        .expect("Q class to be valid")
    }

    pub fn size_in_packet(&self) -> usize {
        4 + self.name().size_in_packet()
    }
}

impl<'data> super::FromPacketBytes<'data> for Question<'data> {
    type Error = QuestionError;

    fn parse(bytes: &'data [u8], offset: usize) -> Result<Option<Self>, Self::Error> {
        let Some(name) = DomainName::parse(&bytes, offset)? else {
            return Ok(None);
        };
        let name_size = name.size_in_packet();
        if offset + name_size + 4 > bytes.len() {
            return Err(QuestionError::EOF);
        }
        Ok(Some(Self {
            buffer: bytes,
            offset,
        }))
    }
}

impl<'a> fmt::Display for Question<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {:?} {:?}",
            self.name(),
            self.q_class(),
            self.q_type()
        )
    }
}

impl<'a> fmt::Debug for Question<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Question")
            .field("name", &self.name())
            .field("q_type", &self.q_type())
            .field("q_class", &self.q_class())
            .finish()
    }
}
