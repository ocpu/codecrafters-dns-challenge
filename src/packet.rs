use crate::{
    header::{Header, HeaderParseError, PacketType, ResponseCode},
    question::{Question, QuestionParseError},
    resource::{Resource, ResourceParseError},
    types::{CowData, DomainName},
};

pub struct DNSPacket<'a> {
    header: Header,
    questions: Box<[Question<'a>]>,
    _answers: Box<[Resource<'a>]>,
}

pub struct DNSPacketBuilder<'a> {
    header: Header,
    questions: Vec<Question<'a>>,
    answers: Vec<Resource<'a>>,
}

#[derive(Debug)]
pub enum DNSPacketParseError {
    Header(HeaderParseError),
    Question(QuestionParseError),
    Answer(ResourceParseError),
}

fn get_entry_vec<T>(entries: u16) -> Vec<T> {
    if entries > 10 {
        vec![]
    } else {
        Vec::with_capacity(entries.into())
    }
}

impl<'a> DNSPacket<'a> {
    pub fn new(id: u16) -> Self {
        Self {
            header: Header::new(id),
            questions: Vec::new().into_boxed_slice(),
            _answers: Vec::new().into_boxed_slice(),
        }
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn questions(&self) -> &[Question<'a>] {
        &self.questions
    }
/*
    pub fn answers(&self) -> &[Resource<'a>] {
        &self.answers
    }
*/
    pub fn try_parse_header_only(buffer: &'a [u8]) -> Option<DNSPacket<'a>> {
        let header = match Header::try_from(buffer) {
            Ok(header) => header,
            Err(_) if buffer.len() >= 2 => Header::new(u16::from_be_bytes([buffer[0], buffer[1]])),
            Err(_) => return None,
        };
        Some(Self {
            header,
            questions: Vec::new().into_boxed_slice(),
            _answers: Vec::new().into_boxed_slice(),
        })
    }

    pub fn try_parse(buffer: &'a [u8]) -> Result<Self, DNSPacketParseError> {
        let header =
            Header::try_from(&buffer[..Header::SIZE]).map_err(DNSPacketParseError::Header)?;

        let mut questions = get_entry_vec(header.question_entries);
        let mut answers = get_entry_vec(header.answer_entries);

        let mut sections = &buffer[Header::SIZE..];

        for _ in 0..header.question_entries {
            let (question, len) =
                Question::try_parse(&sections).map_err(DNSPacketParseError::Question)?;
            sections = &sections[len..];
            questions.push(question);
        }

        for _ in 0..header.answer_entries {
            let (answer, len) =
                Resource::try_parse(&sections).map_err(DNSPacketParseError::Answer)?;
            sections = &sections[len..];
            answers.push(answer);
        }

        Ok(Self {
            header,
            questions: questions.into_boxed_slice(),
            _answers: answers.into_boxed_slice(),
        })
    }

    pub fn respond(&self, code: ResponseCode) -> DNSPacketBuilder<'a> {
        let mut header = Header::new(self.header.id);
        header.opcode = self.header.opcode;
        header.recursion_desired = self.header.recursion_desired;
        header.packet_type = PacketType::Response;
        header.response_code = code;
        DNSPacketBuilder {
            header,
            questions: Vec::new(),
            answers: Vec::new(),
        }
    }
/*
    pub fn respond_ok(&self) -> DNSPacketBuilder<'a> {
        self.respond(ResponseCode::None)
    }

    pub fn query(id: u16) -> DNSPacketBuilder<'a> {
        let header = Header::new(id);
        DNSPacketBuilder {
            header,
            questions: Vec::new(),
            answers: Vec::new(),
        }
    }
*/
}

#[derive(Debug)]
pub enum WritePacketError {
    TooLarge,
}

impl<'a> DNSPacketBuilder<'a> {
    pub fn add_question(mut self, question: Question<'a>) -> Self {
        self.questions.push(question);
        self.header.question_entries += 1;
        self
    }

    pub fn add_answer(mut self, answer: Resource<'a>) -> Self {
        self.answers.push(answer);
        self.header.answer_entries += 1;
        self
    }

    pub fn build_into<'b>(
        self,
        buffer: &'b mut [u8],
    ) -> Result<(DNSPacket<'b>, usize), WritePacketError> {
        self.header.write_into(buffer);

        let mut size = Header::SIZE;
        let mut buffer = &mut buffer[Header::SIZE..];
        let mut questions = Vec::with_capacity(self.questions.len());
        let mut answers = Vec::with_capacity(self.answers.len());

        for question in self.questions {
            if question.len_in_packet() > buffer.len() {
                return Err(WritePacketError::TooLarge);
            }

            // Start write name
            let mut parts = Vec::with_capacity(question.name().parts_count());
            for part in question.name().parts() {
                buffer[0] = part.len() as u8;
                buffer[1..1 + part.len()].copy_from_slice(part.as_bytes());
                let (string, buf) = buffer.split_at_mut(part.len() + 1);
                parts.push(std::str::from_utf8(string).unwrap());
                buffer = buf;
                size += 1 + part.len();
            }
            buffer[0] = 0;
            size += 1;
            // End write name

            buffer = &mut buffer[1..];
            let _ = &buffer[0..2].copy_from_slice(&question.q_type().as_u16().to_be_bytes());
            let _ = &buffer[2..4].copy_from_slice(&question.q_class().as_u16().to_be_bytes());
            buffer = &mut buffer[4..];

            questions.push(Question::new(
                *question.q_type(),
                *question.q_class(),
                DomainName::from_parts(parts.into_boxed_slice()).into(),
            ));
            size += 4;
        }

        for answer in self.answers {
            if answer.len_in_packet() > buffer.len() {
                return Err(WritePacketError::TooLarge);
            }

            // Start write name
            let mut parts = Vec::with_capacity(answer.name().parts_count());
            for part in answer.name().parts() {
                buffer[0] = part.len() as u8;
                buffer[1..1 + part.len()].copy_from_slice(part.as_bytes());
                let (string, buf) = buffer.split_at_mut(part.len() + 1);
                parts.push(std::str::from_utf8(string).unwrap());
                buffer = buf;
                size += 1 + part.len();
            }
            buffer[0] = 0;
            size += 1;
            // End write name

            buffer = &mut buffer[1..];
            let _ = &buffer[0..2].copy_from_slice(&answer.typ().as_u16().to_be_bytes());
            let _ = &buffer[2..4].copy_from_slice(&answer.class().as_u16().to_be_bytes());
            let _ = &buffer[4..8].copy_from_slice(&answer.ttl().to_be_bytes());
            let _ = &buffer[8..10].copy_from_slice(&(answer.data().len() as u16).to_be_bytes());
            let _ = &buffer[10..10 + answer.data().len()].copy_from_slice(answer.data());

            let (data, buf) = buffer.split_at_mut(10 + answer.data().len());
            buffer = buf;

            answers.push(Resource::new(
                DomainName::from_parts(parts.into_boxed_slice()).into(),
                *answer.typ(),
                *answer.class(),
                *answer.ttl(),
                CowData::Borrowed(&data[10..]),
            ));
            size += 10 + answer.data().len();
        }

        Ok((
            DNSPacket {
                header: self.header,
                questions: questions.into_boxed_slice(),
                _answers: answers.into_boxed_slice(),
            },
            size,
        ))
    }
}
