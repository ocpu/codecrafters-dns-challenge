#[derive(Debug)]
pub struct UnknownType(pub u16);

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
