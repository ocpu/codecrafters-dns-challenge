use std::sync::Arc;

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
