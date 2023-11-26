use std::sync::Arc;

#[derive(Debug)]
pub struct UnknownType(pub u16);
/*
71, 203, -- id
1, 0, -- flags
0, 2, -- questions
0, 0, -- answers
0, 0, -- authorative
0, 0, -- additional
-- question 1
-- qname
3, 97, 98, 99, (Len: 3) abc
17, 108, 111, 110, 103, 97, 115, 115, 100, 111, 109, 97, 105, 110, 110, 97, 109, 101, (Len 17) longassdomainname
3, 99, 111, 109, (Len: 3) com
0, (Len: 0)
0, 1, -- qtype
0, 1, -- qclass
-- question 2
-- qname
3, 100, 101, 102, (Len: 3) def
192, 16, 
0, 1, 0, 1]
*/

#[derive(Debug, Clone, Copy)]
pub enum Type {
    A,
    NS,
    #[deprecated]
    MD,
    #[deprecated]
    MF,
    CNAME,
    SOA,
    #[deprecated]
    MB,
    #[deprecated]
    MG,
    #[deprecated]
    MR,
    #[deprecated]
    NULL,
    #[deprecated]
    WKS,
    PTR,
    HINFO,
    MINFO,
    MX,
    TXT,
    RP,
    AAAA,
}

#[derive(Debug, Clone, Copy)]
pub enum QType {
    A,
    NS,
    #[deprecated]
    MD,
    #[deprecated]
    MF,
    CNAME,
    SOA,
    #[deprecated]
    MB,
    #[deprecated]
    MG,
    #[deprecated]
    MR,
    #[deprecated]
    NULL,
    #[deprecated]
    WKS,
    PTR,
    HINFO,
    MINFO,
    MX,
    TXT,
    RP,
    AAAA,
    AXFR,
    #[deprecated]
    MAILB,
    #[deprecated]
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
            Self::RP => 17,
            Self::AAAA => 28,
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
            Self::RP => 16,
            Self::AAAA => 28,
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
            17 => Type::RP,
            28 => Type::AAAA,
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
            Type::RP => Self::RP,
            Type::AAAA => Self::AAAA,
        }
    }
}

#[derive(Debug)]
pub struct UnknownClass(pub u16);

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

#[derive(Debug, Clone)]
pub enum CowData<'a> {
    Owned(Arc<[u8]>),
    Borrowed(&'a [u8]),
}

impl<'a> CowData<'a> {
    pub fn len(&self) -> usize {
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
