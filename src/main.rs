mod header;

use std::{fmt::Display, net::UdpSocket};

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
        Ok((Self {
            // TODO: Make sure that it is only ASCII
            content: std::str::from_utf8(&buffer[..cursor])
                .map_err(|_| QuestionParseError::IllegalName)?,
            offsets: offsets.into_boxed_slice(),
            q_type,
            q_class,
        }, cursor + 4))
    }
}

impl<'a, 'b: 'a> Iterator for QuestionPartIter<'a, 'b> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let Some((offset, len)) = self.q.offsets.get(self.index) else {
            return None;
        };
        let (offset, len) = (*offset, *len);
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

fn main() {
    println!("Logs from your program will appear here!");

    let udp_socket = UdpSocket::bind("127.0.0.1:2053").expect("Failed to bind to address");
    let mut buf = [0; MAX_MESSAGE_SIZE];
    let mut response = [0; MAX_MESSAGE_SIZE];

    'recv: loop {
        let Ok((size, source)) = udp_socket.recv_from(&mut buf) else {
            eprintln!("ERROR: receiving data from socket");
            break;
        };
        println!("Input: {:?}", &buf[..size]);

        let header = match Header::try_from(&buf[..]) {
            Ok(val) => val,
            Err(_) => continue,
        };
        let mut questions = if header.question_entries > 10 {
            vec![]
        } else {
            Vec::with_capacity(header.question_entries.into())
        };

        let mut sections = &buf[Header::SIZE..size];

        for _ in 0..header.question_entries {
            let (question, len) = match Question::try_parse(&sections) {
                Ok(res) => res,
                Err(_) => {
                    let mut res_header = Header::new(header.id);
                    res_header.response_code = ResponseCode::FormatError;
                    res_header.write_into(&mut response[..]);
                    let Ok(_) = udp_socket.send_to(&response[..Header::SIZE], source) else {
                        eprintln!("Failed to send response");
                        continue 'recv;
                    };
                    continue 'recv;
                }
            };
            sections = &sections[len..];
            questions.push(question);
        }

        for q in &questions {
            println!("q = {q}");
        }





        let mut response_header = Header::new(header.id);
        response_header.message_type = MessageType::Response;
        response_header.write_into(&mut response[..]);
        println!("Output: {:?}", &response[..Header::SIZE]);
        udp_socket
            .send_to(&response[..Header::SIZE], source)
            .expect("Failed to send response");
    }
}
