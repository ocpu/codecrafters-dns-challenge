use std::{fmt::Display, sync::Arc};

use crate::types::{QClass, QType};

const MAX_LABEL_SIZE: usize = 63;
const MAX_NAME_SIZE: usize = 255;

pub struct Question<'a> {
    content: &'a str,
    offsets: Box<[(usize, usize)]>,
    q_type: QType,
    q_class: QClass,
}

struct QuestionPartIter<'a, 'b> {
    index: usize,
    q: &'b Question<'a>,
}

pub struct QuestionOwned {
    parts: Arc<[Box<str>]>,
    q_type: QType,
    q_class: QClass,
}

#[derive(Debug)]
pub enum QuestionParseError {
    LabelTooLarge,
    NameTooLarge,
    UnknownQType,
    UnkonwnQClass,
    IllegalName,
    EOF,
}

impl<'a> Question<'a> {
    pub fn new(
        content: &'a str,
        offsets: Box<[(usize, usize)]>,
        q_type: QType,
        q_class: QClass,
    ) -> Self {
        Self {
            content,
            offsets,
            q_type,
            q_class,
        }
    }

    pub fn parts<'b: 'a>(&'b self) -> impl Iterator<Item = &'a str> {
        QuestionPartIter { index: 0, q: self }
    }

    pub fn try_parse(buffer: &'a [u8]) -> Result<(Self, usize), QuestionParseError> {
        let mut cursor = 0;
        let mut len = 0;
        let mut offsets = vec![];
        loop {
            let Some(part_len) = buffer.get(cursor) else {
                return Err(QuestionParseError::EOF);
            };
            let part_len = if *part_len == 0 {
                cursor += 1;
                break;
            } else if len + (*part_len as usize) > MAX_NAME_SIZE {
                return Err(QuestionParseError::NameTooLarge);
            } else if (*part_len as usize) < MAX_LABEL_SIZE {
                part_len.to_owned() as usize
            } else {
                return Err(QuestionParseError::LabelTooLarge);
            };

            offsets.push((cursor + 1, part_len));
            len += part_len;
            cursor += part_len + 1;
        }
        if cursor + 4 > buffer.len() {
            return Err(QuestionParseError::EOF);
        }
        let q_type = QType::try_from(u16::from_be_bytes([buffer[cursor], buffer[cursor + 1]]))
            .map_err(|_| QuestionParseError::UnknownQType)?;
        let q_class =
            QClass::try_from(u16::from_be_bytes([buffer[cursor + 2], buffer[cursor + 3]]))
                .map_err(|_| QuestionParseError::UnkonwnQClass)?;
        Ok((
            Self {
                // TODO: Make sure that it is only ASCII
                content: std::str::from_utf8(&buffer[..cursor])
                    .map_err(|_| QuestionParseError::IllegalName)?,
                offsets: offsets.into_boxed_slice(),
                q_type,
                q_class,
            },
            cursor + 4,
        ))
    }
}

impl QuestionOwned {
    pub fn new(q_type: QType, q_class: QClass, parts: Arc<[Box<str>]>) -> Self {
        Self {
            parts,
            q_type,
            q_class,
        }
    }

    pub fn parts(&self) -> &[Box<str>] {
        &self.parts
    }

    pub fn q_type(&self) -> &QType {
        &self.q_type
    }

    pub fn q_class(&self) -> &QClass {
        &self.q_class
    }
}

impl<'a, 'b: 'a> Iterator for QuestionPartIter<'a, 'b> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let Some((offset, len)) = self.q.offsets.get(self.index) else {
            return None;
        };
        let (offset, len) = (*offset, *len);
        self.index += 1;
        Some(&self.q.content[offset..offset + len])
    }
}

impl<'a> Display for Question<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for item in self.parts() {
            write!(f, "{item}.")?;
        }
        write!(f, " {:?} {:?}", self.q_class, self.q_type)
    }
}
