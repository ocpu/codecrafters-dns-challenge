use crate::{
    header::{Header, HeaderParseError, MessageType, Opcode, ResponseCode},
    question::{Question,  QuestionParseError},
    types::{QClass, QType, DomainName, CowDomainName},
};

pub struct DNSPacket<'a> {
    header: Header,
    questions: Box<[Question<'a>]>,
}

pub struct DNSPacketBuilder<'a> {
    id: u16,
    opcode: Opcode,
    response_code: ResponseCode,
    questions: Vec<Question<'a>>,
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

    pub fn respond(&self, code: ResponseCode) -> DNSPacketBuilder<'a> {
        DNSPacketBuilder {
            id: self.header.id,
            response_code: code,
            opcode: Opcode::Query,
            questions: Vec::new(),
            message_type: MessageType::Response,
        }
    }

    pub fn respond_ok(&self) -> DNSPacketBuilder<'a> {
        self.respond(ResponseCode::None)
    }

    pub fn builder(id: u16) -> DNSPacketBuilder<'a> {
        DNSPacketBuilder {
            id,
            response_code: ResponseCode::None,
            message_type: MessageType::Query,
            opcode: Opcode::Query,
            questions: Vec::new(),
        }
    }
}

impl<'a> DNSPacketBuilder<'a> {
    pub fn add_question(mut self, q_type: QType, q_class: QClass, name: CowDomainName<'a>) -> Self {
        self.questions
            .push(Question::new(name, q_type, q_class));
        self
    }

    pub fn response(mut self, code: ResponseCode) -> Self {
        self.message_type = MessageType::Response;
        self.response_code = code;
        self
    }

    pub fn respone_ok(mut self) -> Self {
        self.message_type = MessageType::Response;
        self.response_code = ResponseCode::None;
        self
    }

    pub fn query(mut self) -> Self {
        self.message_type = MessageType::Query;
        self.response_code = ResponseCode::None;
        self
    }

    pub fn build_into<'b>(self, buffer: &'b mut [u8]) -> (DNSPacket<'b>, usize) {
        let mut header = Header::new(self.id);
        header.message_type = MessageType::Query;
        header.opcode = self.opcode;
        header.message_type = self.message_type;
        header.response_code = self.response_code;
        header.question_entries = self.questions.len() as u16;
        header.write_into(buffer);

        let mut size = Header::SIZE;
        let mut buffer = &mut buffer[Header::SIZE..];
        let mut questions = Vec::with_capacity(self.questions.len());

        for question in self.questions {
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
            buffer = &mut buffer[1..];
            let [b1, b2] = question.q_type().as_u16().to_be_bytes();
            buffer[0] = b1;
            buffer[1] = b2;
            let [b1, b2] = question.q_class().as_u16().to_be_bytes();
            buffer[2] = b1;
            buffer[3] = b2;
            buffer = &mut buffer[4..];
            questions.push(Question::new(
                DomainName::from_parts(parts.into_boxed_slice()).into(),
                *question.q_type(),
                *question.q_class(),
            ));
            size += 4;
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
