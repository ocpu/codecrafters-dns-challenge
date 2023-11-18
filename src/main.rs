use std::net::UdpSocket;

const MAX_LABEL_SIZE: usize = 63;
const MAX_NAME_SIZE: usize = 255;
const MAX_MESSAGE_SIZE: usize = 512;

#[derive(Debug)]
enum Opcode {
    Query,
    InverseQuery,
    Status,
}

impl Opcode {
    const fn as_u8(&self) -> u8 {
        match self {
            Opcode::Query => 0,
            Opcode::InverseQuery => 1,
            Opcode::Status => 2,
        }
    }
}

#[derive(Debug)]
enum MessageType {
    Query,
    Response,
}

impl MessageType {
    const fn as_u8(&self) -> u8 {
        match self {
            MessageType::Query => 0,
            MessageType::Response => 1,
        }
    }
}

#[derive(Debug)]
enum ResponseCode {
    /// No error condition
    None,
    /// The name server was unable to interpret the query.
    FormatError,
    /// The name server was unable to process this query due to
    /// a problem with the name server.
    ServerFailure,
    /// Meaningful only for responses from an authoritative name
    /// server, this code signifies that the domain name referenced
    /// in the query does not exist.
    NameError,
    /// The name server does not support the requested kind of query.
    NotImplemented,
    /// The name server refuses to perform the specified operation
    /// for policy reasons.  For example, a name server may not wish
    /// to provide the information to the particular requester, or a
    /// name server may not wish to perform a particular operation
    /// (e.g., zone transfer) for particular data.
    Refused,
}

impl ResponseCode {
    const fn as_u8(&self) -> u8 {
        match self {
            ResponseCode::None => 0,
            ResponseCode::FormatError => 1,
            ResponseCode::ServerFailure => 2,
            ResponseCode::NameError => 3,
            ResponseCode::NotImplemented => 4,
            ResponseCode::Refused => 5,
        }
    }
}

#[derive(Debug)]
struct Header {
    /// A 16 bit identifier assigned by the program that
    /// generates any kind of query.  This identifier is copied
    /// the corresponding reply and can be used by the requester
    /// to match up replies to outstanding queries.
    ///
    /// Field: ID
    pub id: u16,

    /// A one bit field that specifies whether this message is a
    /// query (0), or a response (1).
    ///
    /// Field: QR
    pub message_type: MessageType,

    /// A four bit field that specifies kind of query in this
    /// message.  This value is set by the originator of a query
    /// and copied into the response.
    ///
    /// Field: Opcode
    pub opcode: Opcode,

    /// Authoritative Answer - this bit is valid in responses,
    /// and specifies that the responding name server is an
    /// authority for the domain name in question section.
    ///
    /// Note that the contents of the answer section may have
    /// multiple owner names because of aliases.  The AA bit
    /// corresponds to the name which matches the query name, or
    /// the first owner name in the answer section.
    ///
    /// Field: AA
    pub authoritive_answer: bool,

    /// TrunCation - specifies that this message was truncated
    /// due to length greater than that permitted on the
    /// transmission channel.
    ///
    /// Field: TC
    pub truncated: bool,

    /// Recursion Desired - this bit may be set in a query and
    /// is copied into the response.  If RD is set, it directs
    /// the name server to pursue the query recursively.
    /// Recursive query support is optional.
    ///
    /// Field: RD
    pub recursion_desired: bool,

    /// Recursion Available - this be is set or cleared in a
    /// response, and denotes whether recursive query support is
    /// available in the name server.
    ///
    /// Field: RA
    pub recursion_available: bool,

    /// Response code - this 4 bit field is set as part of responses.
    ///
    /// Field: RCODE
    pub response_code: ResponseCode,

    /// An unsigned 16 bit integer specifying the number of
    /// entries in the question section.
    ///
    /// Field: QDCOUNT
    pub question_entries: u16,

    /// An unsigned 16 bit integer specifying the number of
    /// resource records in the answer section.
    ///
    /// Field: ANCOUNT
    pub answer_entries: u16,

    /// An unsigned 16 bit integer specifying the number of name
    /// server resource records in the authority records
    /// section.
    ///
    /// Field: NSCOUNT
    pub authority_entries: u16,

    /// An unsigned 16 bit integer specifying the number of
    /// resource records in the additional records section.
    ///
    /// Field: ARCOUNT
    pub additional_entries: u16,
}

impl Header {
    pub const SIZE: usize = 12;
/*
    pub fn builder(id: u16) -> HeaderBuilder {

    }
*/
    pub fn write_into(&self, buffer: &mut [u8]) {
        let [v_1, v_2] = self.id.to_be_bytes();
        buffer[0] = v_1;
        buffer[1] = v_2;
        buffer[2] = self.message_type.as_u8() + (self.opcode.as_u8() << 1) + ((self.authoritive_answer as u8) << 5) + ((self.truncated as u8) << 6) + ((self.recursion_desired as u8) << 7);
        buffer[3] = (self.recursion_available as u8) + (self.response_code.as_u8() << 4);
        let [v_1, v_2] = self.question_entries.to_be_bytes();
        buffer[4] = v_1;
        buffer[5] = v_2;
        let [v_1, v_2] = self.question_entries.to_be_bytes();
        buffer[6] = v_1;
        buffer[7] = v_2;
        let [v_1, v_2] = self.question_entries.to_be_bytes();
        buffer[8] = v_1;
        buffer[9] = v_2;
        let [v_1, v_2] = self.question_entries.to_be_bytes();
        buffer[10] = v_1;
        buffer[11] = v_2;
    }
}

#[derive(Debug)]
enum HeaderParseError {
    UseOfReservedBits,
    UnknownOpcode,
    UnknownResponseCode,
}

impl TryFrom<&[u8]> for Header {
    type Error = HeaderParseError;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if (value[3] & 0b1110) != 0 {
            return Err(HeaderParseError::UseOfReservedBits);
        }
        Ok(Self {
            id: u16::from_be_bytes([value[0], value[1]]),
            message_type: if (value[2] & 1) == 1 {
                MessageType::Response
            } else {
                MessageType::Query
            },
            opcode: match (value[2] & 30).rotate_right(1) {
                0 => Opcode::Query,
                1 => Opcode::InverseQuery,
                2 => Opcode::Status,
                _ => return Err(HeaderParseError::UnknownOpcode),
            },
            authoritive_answer: (value[2] & 32) == 32,
            truncated: (value[2] & 64) == 64,
            recursion_desired: (value[2] & 128) == 128,
            recursion_available: (value[3] & 1) == 1,
            response_code: match (value[3] & 0xf0).rotate_right(4) {
                0 => ResponseCode::None,
                1 => ResponseCode::FormatError,
                2 => ResponseCode::ServerFailure,
                3 => ResponseCode::NameError,
                4 => ResponseCode::NotImplemented,
                5 => ResponseCode::Refused,
                _ => return Err(HeaderParseError::UnknownResponseCode),
            },
            question_entries: u16::from_be_bytes([value[4], value[5]]),
            answer_entries: u16::from_be_bytes([value[6], value[7]]),
            authority_entries: u16::from_be_bytes([value[8], value[9]]),
            additional_entries: u16::from_be_bytes([value[10], value[11]]),
       })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_serde() {
        let input_bytes = [0b100u8, 0b11010010, 0b00000001, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let input_header = Header::try_from(&input_bytes[..]).unwrap();
        println!("{input_header:?}");
        let mut output_bytes = [0u8; Header::SIZE];
        input_header.write_into(&mut output_bytes[..]);
        assert_eq!(&input_bytes[..], &output_bytes[..]);
    }
}

fn main() {
    println!("Logs from your program will appear here!");

    let udp_socket = UdpSocket::bind("127.0.0.1:2053").expect("Failed to bind to address");
    let mut buf = [0; MAX_MESSAGE_SIZE];

    loop {
        match udp_socket.recv_from(&mut buf) {
            Ok((size, source)) => {
                let header = match Header::try_from(&buf[..]) {
                    Ok(val) => val,
                    Err(_) => continue,
                };
                println!("Input header: {header:?}");
                println!("Rest: {:?}", &buf[Header::SIZE..size]);
                println!("All: {:?}", &buf[..size]);
                let mut response = [0; MAX_MESSAGE_SIZE];
                let response_header = Header {
                    id: header.id,
                    message_type: MessageType::Response,
                    opcode: Opcode::Query,
                    authoritive_answer: false,
                    truncated: false,
                    recursion_desired: false,
                    recursion_available: false,
                    response_code: ResponseCode::None,
                    question_entries: 0,
                    answer_entries: 0,
                    authority_entries: 0,
                    additional_entries: 0,
                };
                response_header.write_into(&mut response[..]);
                udp_socket
                    .send_to(&response[..0], source)
                    .expect("Failed to send response");
            }
            Err(e) => {
                eprintln!("Error receiving data: {}", e);
                break;
            }
        }
    }
}
