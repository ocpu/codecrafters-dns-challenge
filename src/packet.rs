use crate::{
    domain_name::DomainName,
    header::{Header, HeaderParseError, PacketType, ResponseCode},
    label::Label,
    question::{Question, QuestionParseError},
    resource::{Resource, ResourceParseError},
    types::CowData,
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
    compress: bool,
}

#[derive(Debug)]
pub enum DNSPacketParseError {
    Header(HeaderParseError),
    NoQuestionFound,
    Question(QuestionParseError),
    NoAnswerFound,
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

        let mut offset = Header::SIZE;

        for _ in 0..header.question_entries {
            let (question, len) = Question::try_parse(&buffer, offset)
                .map_err(DNSPacketParseError::Question)?
                .ok_or(DNSPacketParseError::NoQuestionFound)?;
            offset += len;
            questions.push(question);
        }

        for _ in 0..header.answer_entries {
            let (answer, len) = Resource::try_parse(&buffer, offset)
                .map_err(DNSPacketParseError::Answer)?
                .ok_or(DNSPacketParseError::NoAnswerFound)?;
            offset += len;
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
            compress: true,
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

    pub fn disable_compression(mut self) -> Self {
        self.compress = false;
        self
    }

    pub fn build_into<'b>(
        self,
        out_buffer: &'b mut [u8],
    ) -> Result<(DNSPacket<'b>, usize), WritePacketError> {
        self.header.write_into(out_buffer);

        let mut size = Header::SIZE;
        let mut buffer = &mut out_buffer[Header::SIZE..];
        let mut questions = Vec::with_capacity(self.questions.len());
        let mut answers = Vec::with_capacity(self.answers.len());
        let mut written_names: Vec<(DomainName<'b>, usize)> = Vec::new();

        for question in self.questions {
            if question.len_in_packet() > buffer.len() {
                return Err(WritePacketError::TooLarge);
            }

            let (name, buf) = write_name(
                buffer,
                &mut size,
                question.name(),
                self.compress,
                &mut written_names,
            );
            buffer = buf;

            let _ = &buffer[0..2].copy_from_slice(&question.q_type().as_u16().to_be_bytes());
            let _ = &buffer[2..4].copy_from_slice(&question.q_class().as_u16().to_be_bytes());
            buffer = &mut buffer[4..];

            questions.push(Question::new(*question.q_type(), *question.q_class(), name));
            size += 4;
        }

        for answer in self.answers {
            if answer.len_in_packet() > buffer.len() {
                return Err(WritePacketError::TooLarge);
            }

            let (name, buf) = write_name(
                buffer,
                &mut size,
                answer.name(),
                self.compress,
                &mut written_names,
            );
            buffer = buf;
            let _ = &buffer[0..2].copy_from_slice(&answer.typ().as_u16().to_be_bytes());
            let _ = &buffer[2..4].copy_from_slice(&answer.class().as_u16().to_be_bytes());
            let _ = &buffer[4..8].copy_from_slice(&answer.ttl().to_be_bytes());
            let _ = &buffer[8..10].copy_from_slice(&(answer.data().len() as u16).to_be_bytes());
            let _ = &buffer[10..10 + answer.data().len()].copy_from_slice(answer.data());

            let (data, buf) = buffer.split_at_mut(10 + answer.data().len());
            buffer = buf;

            answers.push(Resource::new(
                name,
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

fn write_name<'a, 'b>(
    mut buffer: &'b mut [u8],
    size: &mut usize,
    domain_name: &DomainName<'a>,
    compress: bool,
    written_names: &mut Vec<(DomainName<'b>, usize)>,
) -> (DomainName<'b>, &'b mut [u8]) {
    if compress && written_names.iter().any(|(name, _)| name == domain_name) {
        let (name, offset) = written_names
            .iter()
            .find(|(name, _)| name == domain_name)
            .unwrap();
        buffer[0] = (((*offset >> 8) as u8) & 0x3f) | 0xc0;
        buffer[1] = *offset as u8;
        *size += 2;
        (name.clone(), &mut buffer[2..])
    } else {
        let mut labels = domain_name.labels();
        let mut domain_labels = Vec::with_capacity(domain_name.labels().len());
        let mut pointer = 0;
        loop {
            if labels.is_empty() {
                break;
            }
            // TODO: Check available space
            buffer[0] = labels[0].len() as u8;
            buffer[1..1 + labels[0].len()].copy_from_slice(labels[0].as_bytes());

            let (string, buf) = buffer.split_at_mut(labels[0].len() + 1);
            domain_labels.push(Label::new(std::str::from_utf8(&string[1..]).unwrap()));
            buffer = buf;
            *size += 1 + labels[0].len();

            labels = &labels[1..];
            if labels.is_empty() {
                break;
            }
            if compress {
                let next_name: DomainName<'a> = DomainName::new(labels.into());
                if let Some((name, offset)) =
                    written_names.iter().find(|(name, _)| name == &next_name)
                {
                    pointer = name.labels().len();
                    buffer[0] = (((*offset >> 8) as u8) & 0x3f) | 0xc0;
                    buffer[1] = *offset as u8;
                    let name = name.clone();
                    for label in name.labels() {
                        domain_labels.push(label.clone());
                    }
                    buffer = &mut buffer[2..];
                    *size += 2;
                    break;
                }
            }
        }
        if pointer == 0 {
            buffer[0] = 0;
            *size += 1;
            buffer = &mut buffer[1..];
        }

        // 1 => 1..=1 = 1     | [1 - 1..]
        // 2 => 1..=2 = 1, 2  | [2 - 1..], [2 - 2..]
        for i in 1..=domain_labels.len() - pointer {
            let labels = &domain_labels[domain_labels.len() - pointer - i..];

            // The 1 here refers to the last "null" in the name sequence.
            // While the 1 in the map refers to the length byte.
            let buffer_len: usize = 1 + labels[..labels.len() - pointer]
                .iter()
                .map(|label| label.len() + 1)
                .sum::<usize>();
            written_names.push((DomainName::new(labels.into()), *size - buffer_len))
        }

        (DomainName::new(domain_labels.into()), buffer)
    }
}
