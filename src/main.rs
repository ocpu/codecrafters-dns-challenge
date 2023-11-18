mod header;

use std::{fmt::Display, net::UdpSocket, sync::Arc};

use header::{HeaderParseError, Opcode};

use crate::header::{Header, MessageType, ResponseCode};

const MAX_LABEL_SIZE: usize = 63;
const MAX_NAME_SIZE: usize = 255;
const MAX_MESSAGE_SIZE: usize = 512;

#[derive(Debug)]
struct UnknownType(u16);

#[derive(Debug)]
enum Type {
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

#[derive(Debug)]
enum QType {
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
struct UnknownClass(u16);

#[derive(Debug)]
enum Class {
    IN,
    CS,
    CH,
    HS,
}

#[derive(Debug)]
enum QClass {
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

struct Question<'a> {
    content: &'a str,
    offsets: Box<[(usize, usize)]>,
    q_type: QType,
    q_class: QClass,
}

struct QuestionPartIter<'a, 'b> {
    index: usize,
    q: &'b Question<'a>,
}

#[derive(Debug)]
enum QuestionParseError {
    LabelTooLarge,
    NameTooLarge,
    UnknownQType,
    UnkonwnQClass,
    IllegalName,
    EOF,
}

impl<'a> Question<'a> {
    fn parts<'b: 'a>(&'b self) -> impl Iterator<Item = &'a str> {
        QuestionPartIter { index: 0, q: self }
    }

    fn try_parse(buffer: &'a [u8]) -> Result<(Self, usize), QuestionParseError> {
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

struct DNSPacket<'a> {
    header: Header,
    questions: Box<[Question<'a>]>,
}

enum DNSPacketParseError {
    Header(HeaderParseError),
    Question(QuestionParseError),
}

impl<'a> DNSPacket<'a> {
    fn try_parse(buffer: &'a [u8]) -> Result<Self, DNSPacketParseError> {
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

    fn respond(&self) -> DNSPacketBuilder {
        DNSPacketBuilder {
            id: self.header.id,
            opcode:Opcode::Query, questions: Vec::new(),    message_type: MessageType::Query,
    }
    }

    fn builder(id: u16) -> DNSPacketBuilder {
        DNSPacketBuilder {
            id,
            message_type: MessageType::Query,
                            opcode: Opcode::Query,
                questions: Vec::new(),
                   }
    }
}

struct QuestionOwned {
    parts: Arc<[Box<str>]>,
    q_type: QType,
    q_class: QClass,
}

struct DNSPacketBuilder {
    id: u16,
    opcode: Opcode,
    questions: Vec<QuestionOwned>,
    message_type: MessageType,
}

impl DNSPacketBuilder {
    pub fn add_question(mut self, q_type: QType, q_class: QClass, parts: Arc<[Box<str>]>) -> Self {
        self.questions.push(QuestionOwned {
            parts,
            q_type,
            q_class,
        });
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
            let mut offsets = Vec::with_capacity(question.parts.len());
            for part in question.parts.as_ref() {
                buffer[offset] = part.len() as u8;
                buffer[offset + 1..offset + 1 + part.len()].copy_from_slice(part.as_bytes());
                offsets.push((offset + 1, part.len()));
                offset += part.len() + 1;
            }
            offset += 1;
            buffer[offset] = 0;
            let (content, buf) = buffer.split_at_mut(offset);
            buffer = buf;
            let [b1, b2] = question.q_type.as_u16().to_be_bytes();
            buffer[0] = b1;
            buffer[1] = b2;
            let [b1, b2] = question.q_class.as_u16().to_be_bytes();
            buffer[2] = b1;
            buffer[3] = b2;
            buffer = &mut buffer[4..];
            let content = std::str::from_utf8(content).unwrap();
            questions.push(Question {
                content,
                offsets: offsets.into_boxed_slice(),
                q_type: question.q_type,
                q_class: question.q_class,
            });
            size += offset + 4;
        }

        (DNSPacket {
            header,
            questions: questions.into_boxed_slice(),
        }, size)
    }
}

fn main() {
    println!("Logs from your program will appear here!");

    let udp_socket = UdpSocket::bind("127.0.0.1:2053").expect("Failed to bind to address");
    let mut buf = [0; MAX_MESSAGE_SIZE];
    let mut response = [0; MAX_MESSAGE_SIZE];

    loop {
        let Ok((size, source)) = udp_socket.recv_from(&mut buf) else {
            eprintln!("ERROR: receiving data from socket");
            break;
        };
        println!("Input: {:?}", &buf[..size]);
        let Ok(packet) = DNSPacket::try_parse(&buf[..size]) else {
            let mut res_header = Header::new(u16::from_be_bytes([buf[0], buf[1]]));
            res_header.response_code = ResponseCode::FormatError;
            res_header.write_into(&mut response[..]);
            let Ok(_) = udp_socket.send_to(&response[..Header::SIZE], source) else {
                eprintln!("Failed to send response");
                continue;
            };
            continue;
        };

        for q in packet.questions.iter() {
            println!("q = {q}");
        }

        let (_response_header, resp_size) = DNSPacket::builder(1234)
            .response()
            .add_question(
                QType::A,
                QClass::IN,
                Arc::from(vec!["codecrafters".into(), "io".into()]),
            )
            .build_into(&mut response[..]);

        println!("Output: {:?}", &response[..resp_size]);
        udp_socket
            .send_to(&response[..resp_size], source)
            .expect("Failed to send response");
    }
}
