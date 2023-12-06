use std::{fmt::Display, hash::Hash};

use thiserror::Error;

use super::FromPacketBytes;

const MAX_LABEL_LENGTH: usize = 63;

#[derive(Debug, Clone, Copy)]
pub enum Label<'data> {
    Data {
        data: &'data str,
        offset: usize,
        buffer: &'data [u8],
    },
    Pointer {
        offset: usize,
        buffer: &'data [u8],
    },
}

#[derive(Debug, Error)]
pub enum LabelError {
    #[error("The length specified for the label is too long. Specified length is {0} expected a max length of {MAX_LABEL_LENGTH}.")]
    LabelLengthTooLong(usize),
    #[error("The amount of remaining bytes in the buffer ({remaining}) is not enough for the label ({expected})")]
    BufferTooSmall { remaining: usize, expected: usize },
    #[error(
        "The character code of {0:x?} is not allowed in a label. Expected A-Z, a-z, 0-9, and -."
    )]
    IllegalLabelChar(u8),
    #[error("The label pointer does not point to a label. (Pointer -> {0})")]
    IllegalLabelPointer(u16),
    #[error("The length field specified has set either of the 2 upper bits")]
    InvalidLengthField(u8),
}

impl<'data> Label<'data> {
    pub fn data(&self) -> Option<&'data str> {
        match self {
            Self::Data { data, .. } => Some(data),
            _ => None,
        }
    }
}

impl<'data> super::FromPacketBytes<'data> for Label<'data> {
    type Error = LabelError;

    fn parse(bytes: &'data [u8], offset: usize) -> Result<Option<Self>, Self::Error> {
        use LabelError::*;

        let Some(&len) = bytes.get(offset) else {
            return Err(BufferTooSmall {
                remaining: 0,
                expected: 1,
            });
        };
        let len = if len == 0 {
            return Ok(None);
        } else if (len & 0xc0) == 0xc0 {
            let offset = u16::from_be_bytes([
                len & 0x3f,
                *bytes.get(offset + 1).ok_or_else(|| BufferTooSmall {
                    remaining: bytes.len() - offset,
                    expected: 2,
                })?,
            ]);
            if offset as usize >= bytes.len() {
                return Err(IllegalLabelPointer(offset));
            }
            return Ok(Some(Self::Pointer {
                offset: offset as usize,
                buffer: bytes,
            }));
        } else if offset + 1 + (len as usize) > bytes.len() {
            return Err(BufferTooSmall {
                remaining: bytes.len() - offset,
                expected: len as usize,
            });
        } else if (len as usize) > MAX_LABEL_LENGTH {
            return Err(LabelLengthTooLong(len as usize));
        } else if (len & 0xc0) != 0 {
            return Err(InvalidLengthField(len));
        } else {
            len as usize
        };

        let mut cursor = 0;
        while cursor < len {
            match &bytes[offset + 1 + cursor] {
                b'A'..=b'Z' | b'a'..=b'z' => {}
                b'0'..=b'9' if cursor != 0 => {}
                b'-' if cursor != 0 && cursor + 1 != len => {}
                c => return Err(IllegalLabelChar(*c)),
            }
            cursor += 1;
        }

        Ok(Some(Self::Data {
            // SAFETY: All chars has already been validated to be ascii and as ascii is a valid
            // subset of UTF-8 then this is correct.
            data: unsafe { std::str::from_utf8_unchecked(&bytes[offset + 1..offset + 1 + len]) },
            offset,
            buffer: bytes,
        }))
    }
}

impl<'data> IntoIterator for Label<'data> {
    type Item = Result<Label<'data>, LabelError>;
    type IntoIter = LabelIter<'data>;

    fn into_iter(self) -> Self::IntoIter {
        LabelIter {
            yielded_self: false,
            label: Some(self),
            pointer_mask: 0,
        }
    }
}

pub struct LabelIter<'data> {
    yielded_self: bool,
    label: Option<Label<'data>>,
    pointer_mask: u16,
}

impl<'data> Iterator for LabelIter<'data> {
    type Item = Result<Label<'data>, LabelError>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.yielded_self {
            self.yielded_self = true;
            return self.label.map(Ok);
        }
        let next = match self.label? {
            Label::Data {
                data,
                offset,
                buffer,
            } => Label::parse(buffer, offset + 1 + data.len()),
            Label::Pointer { offset, .. } if (self.pointer_mask & (1u16 << offset)) != 0 => {
                Err(LabelError::IllegalLabelPointer(offset as u16))
            }
            Label::Pointer { offset, buffer } => Label::parse(buffer, offset),
        };
        if let Ok(Some(label)) = &next {
            self.label = Some(*label);
        } else if let Ok(None) = &next {
            self.label = None;
        }
        next.transpose()
    }
}

impl<'data> Display for Label<'data> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Label::Data { data, .. } => write!(f, "{data}."),
            Label::Pointer { .. } => Ok(()),
        }
    }
}

impl<'data> Hash for Label<'data> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Label::Data { data, .. } => {
                for c in data.as_bytes() {
                    state.write_u8(match c {
                        b'A'..=b'Z' | b'a'..=b'z' => c & 0b01011111,
                        c => *c,
                    });
                }
            }
            Label::Pointer { .. } => {}
        }
    }
}
