use std::sync::Arc;

use crate::{
    header::{Header, HeaderParseError, MessageType, Opcode},
    question::{Question, QuestionOwned, QuestionParseError},
    types::{QClass, QType},
};

pub struct DNSPacket<'a> {
    header: Header,
    questions: Box<[Question<'a>]>,
}

pub struct DNSPacketBuilder {
    id: u16,
    opcode: Opcode,
    questions: Vec<QuestionOwned>,
    message_type: MessageType,
}

pub enum DNSPacketParseError {
    Header(HeaderParseError),
    Question(QuestionParseError),
}

impl<'a> DNSPacket<'a> {
    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn questions(&self) -> &[Question<'a>] {
        &self.questions
    }

    pub fn try_parse(buffer: &'a [u8]) -> Result<Self, DNSPacketParseError> {
        let header =
            Header::try_from(&buffer[..Header::SIZE]).map_err(DNSPacketParseError::Header)?;

        let mut questions = if header.question_entries > 10 {
            vec![]
        } else {
            Vec::with_capacity(header.question_entries.into())
        };

        let mut sections = &buffer[Header::SIZE..];

        for _ in 0..header.question_entries {
            let (question, len) =
                Question::try_parse(&sections).map_err(DNSPacketParseError::Question)?;
            sections = &sections[len..];
            questions.push(question);
        }

        Ok(Self {
            header,
            questions: questions.into_boxed_slice(),
        })
    }

    pub fn respond(&self) -> DNSPacketBuilder {
        DNSPacketBuilder {
            id: self.header.id,
            opcode: Opcode::Query,
            questions: Vec::new(),
            message_type: MessageType::Query,
        }
    }

    pub fn builder(id: u16) -> DNSPacketBuilder {
        DNSPacketBuilder {
            id,
            message_type: MessageType::Query,
            opcode: Opcode::Query,
            questions: Vec::new(),
        }
    }
}

impl DNSPacketBuilder {
    pub fn add_question(mut self, q_type: QType, q_class: QClass, parts: Arc<[Box<str>]>) -> Self {
        self.questions
            .push(QuestionOwned::new(q_type, q_class, parts));
        self
    }

    pub fn response(mut self) -> Self {
        self.message_type = MessageType::Response;
        self
    }

    pub fn query(mut self) -> Self {
        self.message_type = MessageType::Query;
        self
    }

    pub fn build_into<'a>(self, buffer: &'a mut [u8]) -> (DNSPacket<'a>, usize) {
        let mut header = Header::new(self.id);
        header.message_type = MessageType::Query;
        header.opcode = self.opcode;
        header.message_type = self.message_type;
        header.question_entries = self.questions.len() as u16;
        header.write_into(buffer);

        let mut size = Header::SIZE;
        let mut buffer = &mut buffer[Header::SIZE..];
        let mut questions = Vec::with_capacity(self.questions.len());

        for question in self.questions {
            let mut offset = 0;
            let mut offsets = Vec::with_capacity(question.parts().len());
            for part in question.parts() {
                buffer[offset] = part.len() as u8;
                buffer[offset + 1..offset + 1 + part.len()].copy_from_slice(part.as_bytes());
                offsets.push((offset + 1, part.len()));
                offset += part.len() + 1;
            }
            offset += 1;
            buffer[offset] = 0;
            let (content, buf) = buffer.split_at_mut(offset);
            buffer = buf;
            let [b1, b2] = question.q_type().as_u16().to_be_bytes();
            buffer[0] = b1;
            buffer[1] = b2;
            let [b1, b2] = question.q_class().as_u16().to_be_bytes();
            buffer[2] = b1;
            buffer[3] = b2;
            buffer = &mut buffer[4..];
            let content = std::str::from_utf8(content).unwrap();
            questions.push(Question::new(
                content,
                offsets.into_boxed_slice(),
                *question.q_type(),
                *question.q_class(),
            ));
            size += offset + 4;
        }

        (
            DNSPacket {
                header,
                questions: questions.into_boxed_slice(),
            },
            size,
        )
    }
}
