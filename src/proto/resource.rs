use thiserror::Error;

use std::fmt;

use super::{
    class::Class, domain_name::DomainName, label::LabelError, types::Type, DebugList,
    FromPacketBytes,
};

#[derive(Clone, Copy)]
pub struct Resource<'data> {
    pub(super) offset: usize,
    pub(super) buffer: &'data [u8],
}

#[derive(Debug, Error)]
pub enum ResourceError {
    #[error(transparent)]
    Label(#[from] LabelError),
    #[error("TODO")]
    EOF,
}

impl<'data> Resource<'data> {
    pub fn name(&self) -> DomainName<'data> {
        DomainName::parse(self.buffer, self.offset)
            .expect("Domain name to be checked before an instance of Question was created")
            .expect("Domain name to be present")
    }

    pub fn typ(&self) -> Type {
        let name_size = self.name().size_in_packet();
        Type::try_from(u16::from_be_bytes([
            self.buffer[self.offset + name_size],
            self.buffer[self.offset + name_size + 1],
        ]))
        .expect("Type to be valid")
    }

    pub fn class(&self) -> Class {
        let name_size = self.name().size_in_packet();
        Class::try_from(u16::from_be_bytes([
            self.buffer[self.offset + name_size + 2],
            self.buffer[self.offset + name_size + 3],
        ]))
        .expect("Class to be valid")
    }

    pub fn ttl(&self) -> u32 {
        let name_size = self.name().size_in_packet();
        u32::from_be_bytes([
            self.buffer[self.offset + name_size + 4],
            self.buffer[self.offset + name_size + 5],
            self.buffer[self.offset + name_size + 6],
            self.buffer[self.offset + name_size + 7],
        ])
    }

    pub fn data_len(&self) -> usize {
        let name_size = self.name().size_in_packet();
        u16::from_be_bytes([
            self.buffer[self.offset + name_size + 8],
            self.buffer[self.offset + name_size + 9],
        ]) as usize
    }

    pub fn data(&self) -> &'data [u8] {
        let name_size = self.name().size_in_packet();
        let start = self.offset + name_size + 10;
        let data_len = u16::from_be_bytes([
            self.buffer[self.offset + name_size + 8],
            self.buffer[self.offset + name_size + 9],
        ]) as usize;
        &self.buffer[start..start + data_len]
    }

    pub fn size_in_packet(&self) -> usize {
        let name_size = self.name().size_in_packet();
        let data_len = u16::from_be_bytes([
            self.buffer[self.offset + name_size + 8],
            self.buffer[self.offset + name_size + 9],
        ]) as usize;
        10 + name_size + data_len
    }
}

impl<'data> FromPacketBytes<'data> for Resource<'data> {
    type Error = ResourceError;

    fn parse(bytes: &'data [u8], offset: usize) -> Result<Option<Self>, Self::Error> {
        let Some(name) = DomainName::parse(bytes, offset)? else {
            return Ok(None);
        };
        let name_size = name.size_in_packet();
        if offset + name_size + 10 > bytes.len() {
            return Err(ResourceError::EOF);
        }
        let data_length =
            u16::from_be_bytes([bytes[offset + name_size + 8], bytes[offset + name_size + 9]])
                as usize;
        if offset + name_size + 10 + data_length > bytes.len() {
            return Err(ResourceError::EOF);
        }
        Ok(Some(Self {
            buffer: bytes,
            offset,
        }))
    }
}

impl<'a> fmt::Display for Resource<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {:?} {:?}",
            self.name(),
            self.ttl(),
            self.class(),
            self.typ()
        )
    }
}

impl<'a> fmt::Debug for Resource<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Resource")
            .field("name", &self.name())
            .field("ttl", &self.ttl())
            .field("type", &self.typ())
            .field("class", &self.class())
            .field("data_len", &self.data_len())
            .field("data", &DebugList(|| self.data().iter()))
            .finish()
    }
}
