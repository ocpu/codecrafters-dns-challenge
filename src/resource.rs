use std::{net::Ipv4Addr, sync::Arc};

use crate::{
    domain_name::DomainName,
    proto::{Class, Type},
    types::CowData,
};

pub struct Resource(pub DomainName, pub Arc<ResourceData>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResourceData {
    A {
        ttl: u32,
        addr: Ipv4Addr,
    },
    //AAAA {
    //    ttl: u32,
    //    addr: Ipv6Addr,
    //},
    Generic {
        typ: Type,
        class: Class,
        ttl: u32,
        data: Arc<[u8]>,
    },
}

impl ResourceData {
    pub fn class(&self) -> &Class {
        match self {
            Self::A { .. } => &Class::IN,
            //Self::AAAA { .. } => &Class::IN,
            Self::Generic { class, .. } => class,
        }
    }

    pub fn typ(&self) -> &Type {
        match self {
            Self::A { .. } => &Type::A,
            //Self::AAAA { .. } => &Type::AAAA,
            Self::Generic { typ, .. } => typ,
        }
    }

    pub fn ttl(&self) -> &u32 {
        match self {
            Self::A { ttl, .. } => ttl,
            //Self::AAAA { ttl, .. } => ttl,
            Self::Generic { ttl, .. } => ttl,
        }
    }

    pub fn data(&self) -> CowData<'_> {
        match self {
            Self::A { addr, .. } => CowData::Owned(Arc::from(addr.octets())),
            //Self::AAAA { addr, .. } => &addr.octets(),
            Self::Generic { data, .. } => CowData::Owned(Arc::clone(&data)),
        }
    }
}

impl<'data> From<crate::proto::Resource<'data>> for ResourceData {
    fn from(value: crate::proto::Resource<'data>) -> Self {
        ResourceData::Generic {
            typ: value.typ(),
            class: value.class(),
            ttl: value.ttl(),
            data: Arc::from(value.data()),
        }
    }
}
