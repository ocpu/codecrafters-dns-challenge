use std::{fmt::Display, hash::Hash, marker::PhantomData, ops::Deref, sync::Arc};

#[derive(Debug)]
pub struct UnknownType(u16);

#[derive(Debug, Clone, Copy)]
pub enum Type {
    A,
    NS,
    MD,
    MF,
    CNAME,
    SOA,
    MB,
    MG,
    MR,
    NULL,
    WKS,
    PTR,
    HINFO,
    MINFO,
    MX,
    TXT,
}

#[derive(Debug, Clone, Copy)]
pub enum QType {
    A,
    NS,
    MD,
    MF,
    CNAME,
    SOA,
    MB,
    MG,
    MR,
    NULL,
    WKS,
    PTR,
    HINFO,
    MINFO,
    MX,
    TXT,
    AXFR,
    MAILB,
    MAILA,
    All,
}

impl Type {
    pub const fn as_u16(&self) -> u16 {
        match self {
            Self::A => 1,
            Self::NS => 2,
            Self::MD => 3,
            Self::MF => 4,
            Self::CNAME => 5,
            Self::SOA => 6,
            Self::MB => 7,
            Self::MG => 8,
            Self::MR => 9,
            Self::NULL => 10,
            Self::WKS => 11,
            Self::PTR => 12,
            Self::HINFO => 13,
            Self::MINFO => 14,
            Self::MX => 15,
            Self::TXT => 16,
        }
    }
}

impl QType {
    pub const fn as_u16(&self) -> u16 {
        match self {
            Self::A => 1,
            Self::NS => 2,
            Self::MD => 3,
            Self::MF => 4,
            Self::CNAME => 5,
            Self::SOA => 6,
            Self::MB => 7,
            Self::MG => 8,
            Self::MR => 9,
            Self::NULL => 10,
            Self::WKS => 11,
            Self::PTR => 12,
            Self::HINFO => 13,
            Self::MINFO => 14,
            Self::MX => 15,
            Self::TXT => 16,
            Self::AXFR => 252,
            Self::MAILB => 253,
            Self::MAILA => 254,
            Self::All => 255,
        }
    }
}

impl TryFrom<u16> for Type {
    type Error = UnknownType;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Ok(match value {
            1 => Type::A,
            2 => Type::NS,
            3 => Type::MD,
            4 => Type::MF,
            5 => Type::CNAME,
            6 => Type::SOA,
            7 => Type::MB,
            8 => Type::MG,
            9 => Type::MR,
            10 => Type::NULL,
            11 => Type::WKS,
            12 => Type::PTR,
            13 => Type::HINFO,
            14 => Type::MINFO,
            15 => Type::MX,
            16 => Type::TXT,
            val => return Err(UnknownType(val)),
        })
    }
}

impl TryFrom<u16> for QType {
    type Error = UnknownType;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Ok(match Type::try_from(value) {
            Ok(typ) => QType::from(typ),
            Err(UnknownType(val)) => match val {
                252 => QType::AXFR,
                253 => QType::MAILB,
                254 => QType::MAILA,
                255 => QType::All,
                val => return Err(UnknownType(val)),
            },
        })
    }
}

impl From<Type> for QType {
    fn from(value: Type) -> Self {
        match value {
            Type::A => Self::A,
            Type::NS => Self::NS,
            Type::MD => Self::MD,
            Type::MF => Self::MF,
            Type::CNAME => Self::CNAME,
            Type::SOA => Self::SOA,
            Type::MB => Self::MB,
            Type::MG => Self::MG,
            Type::MR => Self::MR,
            Type::NULL => Self::NULL,
            Type::WKS => Self::WKS,
            Type::PTR => Self::PTR,
            Type::HINFO => Self::HINFO,
            Type::MINFO => Self::MINFO,
            Type::MX => Self::MX,
            Type::TXT => Self::TXT,
        }
    }
}

#[derive(Debug)]
pub struct UnknownClass(u16);

#[derive(Debug, Clone, Copy)]
pub enum Class {
    IN,
    CS,
    CH,
    HS,
}

#[derive(Debug, Clone, Copy)]
pub enum QClass {
    IN,
    CS,
    CH,
    HS,
    Any,
}

impl Class {
    pub const fn as_u16(&self) -> u16 {
        match self {
            Self::IN => 1,
            Self::CS => 2,
            Self::CH => 3,
            Self::HS => 4,
        }
    }
}

impl QClass {
    pub const fn as_u16(&self) -> u16 {
        match self {
            Self::IN => 1,
            Self::CS => 2,
            Self::CH => 3,
            Self::HS => 4,
            Self::Any => 255,
        }
    }
}

impl TryFrom<u16> for Class {
    type Error = UnknownClass;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Ok(match value {
            1 => Self::IN,
            2 => Self::CS,
            3 => Self::CH,
            4 => Self::HS,
            val => return Err(UnknownClass(val)),
        })
    }
}

impl TryFrom<u16> for QClass {
    type Error = UnknownClass;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Ok(match Class::try_from(value) {
            Ok(typ) => Self::from(typ),
            Err(UnknownClass(val)) => match val {
                255 => Self::Any,
                val => return Err(UnknownClass(val)),
            },
        })
    }
}

impl From<Class> for QClass {
    fn from(value: Class) -> Self {
        match value {
            Class::IN => Self::IN,
            Class::CS => Self::CS,
            Class::CH => Self::CH,
            Class::HS => Self::HS,
        }
    }
}

const MAX_LABEL_SIZE: usize = 63;
const MAX_NAME_SIZE: usize = 255;

pub struct DomainName<'a>(Box<[&'a str]>);
pub struct DomainNameOwned(Box<[Box<str>]>);
pub enum CowDomainName<'a> {
    Borrowed(DomainName<'a>),
    Owned(Arc<DomainNameOwned>),
}

#[derive(Debug)]
pub enum DomainNameParseError {
    Empty,
    EOF,
    NameTooLarge,
    LabelTooLarge,
    IllegalLabel,
}

impl<'a> TryFrom<&'a [u8]> for DomainName<'a> {
    type Error = DomainNameParseError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() == 0 {
            return Err(DomainNameParseError::Empty);
        }
        let mut cursor = 0;
        let mut len = 0;
        let mut parts = Vec::new();
        loop {
            let Some(part_len) = value.get(cursor) else {
                return Err(DomainNameParseError::EOF);
            };
            let part_len = if *part_len == 0 {
                break;
            } else if len + (*part_len as usize) > MAX_NAME_SIZE {
                return Err(DomainNameParseError::NameTooLarge);
            } else if cursor + 1 + (*part_len as usize) > value.len() {
                return Err(DomainNameParseError::EOF);
            } else if (*part_len as usize) < MAX_LABEL_SIZE {
                *part_len as usize
            } else {
                return Err(DomainNameParseError::LabelTooLarge);
            };

            let last_index = cursor + part_len;
            let part_range = cursor + 1..cursor + 1 + part_len;
            for i in part_range {
                match value[i] {
                    b'A'..=b'Z' | b'a'..=b'z' => {}
                    b'0'..=b'9' if i != 0 => {}
                    b'.' if i != 0 && i != last_index => {}
                    _ => return Err(DomainNameParseError::IllegalLabel),
                }
            }

            let part_range = cursor + 1usize..cursor + 1usize + part_len;
            parts.push(std::str::from_utf8(&value[part_range]).unwrap());
            len += part_len;
            cursor += part_len + 1;
        }
        Ok(DomainName(parts.into_boxed_slice()))
    }
}

impl<'a> DomainName<'a> {
    pub fn from_parts(parts: Box<[&'a str]>) -> Self {
        Self(parts)
    }

    pub fn from_str(s: &'a str) -> Result<Self, DomainNameParseError> {
        let bytes = s.as_bytes();
        if bytes.len() > MAX_NAME_SIZE {
            return Err(DomainNameParseError::NameTooLarge);
        }
        if bytes.len() == 0 {
            return Err(DomainNameParseError::Empty);
        }
        let mut parts = Vec::new();
        let mut offset = 0;
        loop {
            let mut i = 0;
            loop {
                if offset + i >= bytes.len() {
                    i -= offset + i - bytes.len();
                    break;
                }
                if i > MAX_LABEL_SIZE {
                    return Err(DomainNameParseError::LabelTooLarge);
                }
                match bytes[offset + i] {
                    b'.' => break,
                    b'A'..=b'Z' | b'a'..=b'z' => {}
                    b'-' | b'0'..=b'9' if i != 0 => {}
                    _ => return Err(DomainNameParseError::IllegalLabel),
                }
                i += 1;
            }
            if i == 0 {
                break;
            }
            let (should_continue, part) = match bytes.get(offset + i) {
                None if bytes[offset + i - 1] == b'-' => {
                    return Err(DomainNameParseError::IllegalLabel)
                }
                Some(b'.') if bytes[offset + i - 1] == b'-' => {
                    return Err(DomainNameParseError::IllegalLabel)
                }
                Some(b'.') => (
                    true,
                    std::str::from_utf8(&bytes[offset..offset + i]).unwrap(),
                ),
                None => (
                    false,
                    std::str::from_utf8(&bytes[offset..offset + i]).unwrap(),
                ),
                Some(_) => return Err(DomainNameParseError::IllegalLabel),
            };
            parts.push(part);
            if should_continue {
                offset += i + 1;
            } else {
                break;
            }
        }
        Ok(DomainName(parts.into_boxed_slice()))
    }

    pub fn into_owned(self) -> DomainNameOwned {
        DomainNameOwned(
            self.0
                .iter()
                .map(|part| Box::from(*part))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        )
    }
}

impl DomainNameOwned {
    pub fn len_in_packet(&self) -> usize {
        1 + self.0.iter().map(|part| part.len() + 1).sum::<usize>()
    }

    pub fn parts(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|part| part.deref().as_ref())
    }

    pub fn parts_count(&self) -> usize {
        self.0.len()
    }
}

impl<'a> DomainName<'a> {
    pub fn len_in_packet(&self) -> usize {
        1 + self.0.iter().map(|part| part.len() + 1).sum::<usize>()
    }

    pub fn parts(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|part| part.as_ref())
    }

    pub fn parts_count(&self) -> usize {
        self.0.len()
    }
}

enum CowPartIter<'a, 'b, O, B> {
    Owned(O, PhantomData<&'a ()>),
    Borrowed(B, PhantomData<&'a ()>, PhantomData<&'b ()>),
}

impl<'a, 'b, O, B> Iterator for CowPartIter<'a, 'b, O, B>
where
    'b: 'a,
    O: Iterator<Item = &'a Box<str>>,
    B: Iterator<Item = &'a &'b str>,
{
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Owned(iter, _) => match iter.next() {
                None => None,
                Some(part) => Some((*part).as_ref()),
            },
            Self::Borrowed(iter, _, _) => iter.next().copied(),
        }
    }
}

impl<'a> CowDomainName<'a> {
    pub fn len_in_packet(&self) -> usize {
        match self {
            Self::Owned(name) => name.len_in_packet(),
            Self::Borrowed(name) => name.len_in_packet(),
        }
    }

    pub fn parts(&self) -> impl Iterator<Item = &str> {
        match self {
            Self::Owned(name) => CowPartIter::Owned(name.0.iter(), PhantomData),
            Self::Borrowed(name) => CowPartIter::Borrowed(name.0.iter(), PhantomData, PhantomData),
        }
    }

    pub fn parts_count(&self) -> usize {
        match self {
            Self::Owned(name) => name.parts_count(),
            Self::Borrowed(name) => name.parts_count(),
        }
    }
}

impl<'a> From<DomainName<'a>> for DomainNameOwned {
    fn from(value: DomainName<'a>) -> Self {
        value.into_owned()
    }
}

impl<'a> From<DomainName<'a>> for CowDomainName<'a> {
    fn from(value: DomainName<'a>) -> Self {
        CowDomainName::Borrowed(value)
    }
}

impl<'a> From<DomainNameOwned> for CowDomainName<'a> {
    fn from(value: DomainNameOwned) -> Self {
        CowDomainName::Owned(Arc::new(value))
    }
}

impl<'a> From<&Arc<DomainNameOwned>> for CowDomainName<'a> {
    fn from(value: &Arc<DomainNameOwned>) -> Self {
        CowDomainName::Owned(Arc::clone(value))
    }
}

impl<'a> From<Arc<DomainNameOwned>> for CowDomainName<'a> {
    fn from(value: Arc<DomainNameOwned>) -> Self {
        CowDomainName::Owned(value)
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

impl Display for DomainNameOwned {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for item in self.0.as_ref() {
            write!(f, "{item}.")?;
        }
        Ok(())
    }
}

impl<'a> Display for CowDomainName<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CowDomainName::Owned(name) => name.fmt(f),
            CowDomainName::Borrowed(name) => name.fmt(f),
        }
    }
}

impl<'a> Hash for DomainName<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for part in self.0.iter() {
            for c in part.as_bytes() {
                state.write_u8(match c {
                    b'A'..=b'Z' | b'a'..=b'z' => c & 0b01011111,
                    c => *c,
                });
            }
        }
    }
}

impl Hash for DomainNameOwned {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for part in self.0.iter() {
            for c in part.as_bytes() {
                state.write_u8(match c {
                    b'A'..=b'Z' | b'a'..=b'z' => c & 0b01011111,
                    c => *c,
                });
            }
        }
    }
}

impl<'a> Hash for CowDomainName<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            CowDomainName::Owned(name) => name.hash(state),
            CowDomainName::Borrowed(name) => name.hash(state),
        }
    }
}

pub enum CowData<'a> {
    Owned(Box<[u8]>),
    Borrowed(&'a [u8]),
}

impl<'a> CowData<'a> {
    pub const fn len(&self) -> usize {
        match self {
            Self::Owned(data) => data.len(),
            Self::Borrowed(data) => data.len(),
        }
    }
}

impl<'a> AsRef<[u8]> for CowData<'a> {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::Owned(data) => data.as_ref(),
            Self::Borrowed(data) => data,
        }
    }
}

impl<'a> From<&'a [u8]> for CowData<'a> {
    fn from(value: &'a [u8]) -> Self {
        CowData::Borrowed(value)
    }
}
