use std::net::Ipv4Addr;

use crate::types::{Class, CowData, CowDomainName, DomainName, DomainNameParseError, Type};

pub struct Resource<'a> {
    name: CowDomainName<'a>,
    typ: Type,
    class: Class,
    ttl: u32,
    data: CowData<'a>,
}

#[derive(Debug)]
pub enum ResourceParseError {
    DomainName(DomainNameParseError),
    UnknownType,
    UnkonwnClass,
    EOF,
}

impl<'a> Resource<'a> {
    pub fn new(
        name: CowDomainName<'a>,
        typ: Type,
        class: Class,
        ttl: u32,
        data: CowData<'a>,
    ) -> Self {
        Self {
            name,
            typ,
            class,
            ttl,
            data,
        }
    }

    pub fn name(&self) -> &CowDomainName<'a> {
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

    pub fn try_parse(buffer: &'a [u8]) -> Result<(Self, usize), ResourceParseError> {
        let name: DomainName<'a> = buffer
            .try_into()
            .map_err(|e| ResourceParseError::DomainName(e))?;
        let size = name.len_in_packet();
        if size + 10 > buffer.len() {
            return Err(ResourceParseError::EOF);
        }
        let typ = Type::try_from(u16::from_be_bytes([buffer[size], buffer[size + 1]]))
            .map_err(|_| ResourceParseError::UnknownType)?;
        let class = Class::try_from(u16::from_be_bytes([buffer[size + 2], buffer[size + 3]]))
            .map_err(|_| ResourceParseError::UnkonwnClass)?;
        let ttl = u32::from_be_bytes([
            buffer[size + 4],
            buffer[size + 5],
            buffer[size + 6],
            buffer[size + 7],
        ]);
        let data_length = u16::from_be_bytes([buffer[size + 8], buffer[size + 9]]) as usize;
        if size + 10 + data_length > buffer.len() {
            return Err(ResourceParseError::EOF);
        }
        Ok((
            Self {
                name: name.into(),
                typ,
                class,
                ttl,
                data: CowData::Borrowed(&buffer[size + 10..size + 10 + data_length]),
            },
            size + 10 + data_length,
        ))
    }
}

pub struct ARecord<'a> {
    name: CowDomainName<'a>,
    ttl: u32,
    addr: Ipv4Addr,
}

impl<'a> ARecord<'a> {
    pub fn new(name: CowDomainName<'a>, ttl: u32, addr: Ipv4Addr) -> Self {
        Self { name, ttl, addr }
    }
}

impl<'a> Into<Resource<'a>> for ARecord<'a> {
    fn into(self) -> Resource<'a> {
        Resource::new(
            self.name,
            Type::A,
            Class::IN,
            self.ttl,
            CowData::Owned(Box::from(self.addr.octets())),
        )
    }
}
