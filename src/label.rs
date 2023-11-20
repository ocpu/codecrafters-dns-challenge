use std::{fmt::Display, hash::Hash};

const MAX_LABEL_SIZE: usize = 63;

#[derive(Debug, Clone)]
pub struct Label<'data>(&'data str);

#[derive(Debug)]
pub enum LabelParseError {
    NoLengthField,
    LabelTooLarge(usize),
    BufferTooSmall {
        remaining_len: usize,
        expected_len: usize,
    },
    IllegalLabelChar(u8),
}

pub enum LabelParseResult<'data> {
    Label(Label<'data>),
    Pointer(usize),
    End,
}

impl<'data> Label<'data> {
    pub const fn new(label: &'data str) -> Self {
        Self(label)
    }

    pub const fn len(&self) -> usize {
        self.0.len()
    }

    pub const fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    pub fn try_parse(
        buffer: &'data [u8],
        offset: usize,
    ) -> Result<(usize, LabelParseResult<'data>), LabelParseError> {
        use LabelParseError::*;
        use LabelParseResult::*;

        let Some(&len) = buffer.get(offset) else {
            return Err(NoLengthField);
        };
        let len = if len == 0 {
            return Ok((1, End));
        } else if offset + 1 + (len as usize) > buffer.len() {
            return Err(BufferTooSmall {
                remaining_len: buffer.len() - offset,
                expected_len: len as usize,
            });
        } else if let Some(pointer) = label_pointer(&len) {
            let pointer_lower = *buffer.get(offset + 1).ok_or_else(|| BufferTooSmall {
                remaining_len: buffer.len() - offset,
                expected_len: 2,
            })? as usize;
            return Ok((2, Pointer((pointer << 8) + pointer_lower)));
        } else if (len as usize) > MAX_LABEL_SIZE {
            return Err(LabelTooLarge(len as usize));
        } else {
            // TODO: Fail if it is either 01xxxxxx or 10xxxxxx
            label_length(len)
        };

        Self::valudate_label(&buffer[offset + 1..offset + 1 + len])?;

        Ok((
            1 + len,
            Label(Self(
                std::str::from_utf8(&buffer[offset + 1..offset + 1 + len])
                    .expect("Ascii to be valid utf8"),
            )),
        ))
    }

    const fn valudate_label(label_bytes: &[u8]) -> Result<(), LabelParseError> {
        use LabelParseError::*;

        let mut i = 0;
        while i < label_bytes.len() {
            match &label_bytes[i] {
                b'A'..=b'Z' | b'a'..=b'z' => {}
                b'0'..=b'9' if i != 0 => {}
                b'-' if i != 0 && i + 1 != label_bytes.len() => {}
                c => return Err(IllegalLabelChar(*c)),
            }
            i += 1;
        }

        Ok(())
    }

    pub fn from_str(str: &'data str) -> Result<LabelParseResult<'data>, LabelParseError> {
        use LabelParseError::*;
        use LabelParseResult::*;

        let bytes = str.as_bytes();
        if bytes.is_empty() {
            return Ok(End);
        }

        let mut i = 0;
        while i <= MAX_LABEL_SIZE {
            if i == MAX_LABEL_SIZE {
                return Err(LabelTooLarge(0));
            }
            if i >= bytes.len() || bytes[i] == b'.' {
                match Self::valudate_label(&bytes[0..i]) {
                    Ok(()) => {}
                    Err(e) => return Err(e),
                };
                return Ok(Label(Self(&str[0..i])));
            }
            i += 1;
        }
        Err(LabelTooLarge(0))
    }
}

pub fn label_pointer(char: &u8) -> Option<usize> {
    if (char & 0xc0) == 0xc0 {
        Some((char & 0x3f) as usize)
    } else {
        None
    }
}

fn label_length(char: u8) -> usize {
    (char & 0x3f) as usize
}

impl<'data> AsRef<str> for Label<'data> {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<'data> Hash for Label<'data> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for c in self.0.as_bytes() {
            state.write_u8(match c {
                b'A'..=b'Z' | b'a'..=b'z' => c & 0b01011111,
                c => *c,
            });
        }
    }
}

impl<'data> PartialEq for Label<'data> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(other.0)
    }
}

impl<'data> Eq for Label<'data> {}

impl<'data> Display for Label<'data> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
