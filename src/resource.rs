use std::net::Ipv4Addr;

use crate::{
    domain_name::{DomainName, DomainNameParseError},
    types::{Class, CowData, Type, UnknownClass, UnknownType},
};

pub struct Resource<'a> {
    name: DomainName<'a>,
    typ: Type,
    class: Class,
    ttl: u32,
    data: CowData<'a>,
}

#[derive(Debug)]
pub enum ResourceParseError {
    DomainName(DomainNameParseError),
    UnknownType(u16),
    UnknownClass(u16),
    EOF,
}

impl From<DomainNameParseError> for ResourceParseError {
    fn from(value: DomainNameParseError) -> Self {
        Self::DomainName(value)
    }
}

impl From<UnknownType> for ResourceParseError {
    fn from(value: UnknownType) -> Self {
        Self::UnknownType(value.0)
    }
}

impl From<UnknownClass> for ResourceParseError {
    fn from(value: UnknownClass) -> Self {
        Self::UnknownClass(value.0)
    }
}

impl<'a> Resource<'a> {
    pub fn new(name: DomainName<'a>, typ: Type, class: Class, ttl: u32, data: CowData<'a>) -> Self {
        Self {
            name,
            typ,
            class,
            ttl,
            data,
        }
    }

    pub fn name(&self) -> &DomainName<'a> {
        &self.name
    }

    pub fn typ(&self) -> &Type {
        &self.typ
    }

    pub fn class(&self) -> &Class {
        &self.class
    }

    pub fn ttl(&self) -> &u32 {
        &self.ttl
    }

    pub fn data(&self) -> &[u8] {
        &self.data.as_ref()
    }

    pub fn len_in_packet(&self) -> usize {
        10 + self.name.len_in_packet() + self.data.len()
    }

    pub fn try_parse(
        buffer: &'a [u8],
        offset: usize,
    ) -> Result<Option<(Self, usize)>, ResourceParseError> {
        let Some((name, name_size)) = DomainName::try_parse(buffer, offset)? else {
            return Ok(None);
        };
        if offset + name_size + 10 > buffer.len() {
            return Err(ResourceParseError::EOF);
        }
        let typ = Type::try_from(u16::from_be_bytes([
            buffer[offset + name_size],
            buffer[offset + name_size + 1],
        ]))?;
        let class = Class::try_from(u16::from_be_bytes([
            buffer[offset + name_size + 2],
            buffer[offset + name_size + 3],
        ]))?;
        let ttl = u32::from_be_bytes([
            buffer[offset + name_size + 4],
            buffer[offset + name_size + 5],
            buffer[offset + name_size + 6],
            buffer[offset + name_size + 7],
        ]);
        let data_length = u16::from_be_bytes([
            buffer[offset + name_size + 8],
            buffer[offset + name_size + 9],
        ]) as usize;
        if offset + name_size + 10 + data_length > buffer.len() {
            return Err(ResourceParseError::EOF);
        }
        Ok(Some((
            Self {
                name: name.into(),
                typ,
                class,
                ttl,
                data: CowData::Borrowed(
                    &buffer[offset + name_size + 10..offset + name_size + 10 + data_length],
                ),
            },
            name_size + 10 + data_length,
        )))
    }
}

pub struct ARecord {
    ttl: u32,
    addr: Ipv4Addr,
}

impl ARecord {
    pub fn new(ttl: u32, addr: Ipv4Addr) -> Self {
        Self { ttl, addr }
    }

    pub fn to_resource<'a>(&self, name: DomainName<'a>) -> Resource<'a> {
        Resource::new(
            name,
            Type::A,
            Class::IN,
            self.ttl,
            CowData::Owned(Box::from(self.addr.octets())),
        )
    }
}
