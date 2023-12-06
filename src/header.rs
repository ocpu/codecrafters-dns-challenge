use bytes::BufMut;

use crate::{
    array_buffer::ArrayBuffer,
    proto::{Opcode, PacketType, ResponseCode},
};

#[derive(Debug)]
pub struct Header {
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
    pub packet_type: PacketType,

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

    pub fn new(id: u16) -> Self {
        Self {
            id,
            packet_type: PacketType::Query,
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
        }
    }

    pub fn write_into(&self, buffer: &mut ArrayBuffer) {
        buffer.put_u16(self.id);
        buffer.put_u8(
            (self.recursion_desired as u8)
                + ((self.truncated as u8) << 1)
                + ((self.authoritive_answer as u8) << 2)
                + (self.opcode.as_u8() << 3)
                + (self.packet_type.as_u8() << 7),
        );
        buffer.put_u8(self.response_code.as_u8() + ((self.recursion_available as u8) << 7));
        buffer.put_u16(self.question_entries);
        buffer.put_u16(self.answer_entries);
        buffer.put_u16(self.authority_entries);
        buffer.put_u16(self.additional_entries);
    }
}

#[derive(Debug)]
pub enum HeaderParseError {
    UseOfReservedBits,
    UnknownOpcode(u8),
    UnknownResponseCode(u8),
    EOF,
}

impl TryFrom<&[u8]> for Header {
    type Error = HeaderParseError;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < Self::SIZE {
            return Err(HeaderParseError::EOF);
        }
        if (value[3] & 0b1110) != 0 {
            return Err(HeaderParseError::UseOfReservedBits);
        }
        Ok(Self {
            id: u16::from_be_bytes([value[0], value[1]]),
            packet_type: if (value[2] & 0x80) == 0x80 {
                PacketType::Response
            } else {
                PacketType::Query
            },
            opcode: match (value[2] >> 3) & 0xf {
                0 => Opcode::Query,
                1 => Opcode::InverseQuery,
                2 => Opcode::Status,
                code => return Err(HeaderParseError::UnknownOpcode(code)),
            },
            authoritive_answer: (value[2] & 4) == 4,
            truncated: (value[2] & 2) == 2,
            recursion_desired: (value[2] & 1) == 1,
            recursion_available: (value[3] & 0xf0) == 0xf0,
            response_code: match value[3] & 0xf {
                0 => ResponseCode::None,
                1 => ResponseCode::FormatError,
                2 => ResponseCode::ServerFailure,
                3 => ResponseCode::NameError,
                4 => ResponseCode::NotImplemented,
                5 => ResponseCode::Refused,
                code => return Err(HeaderParseError::UnknownResponseCode(code)),
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
        let input_bytes = [4u8, 210, 16, 0, 0, 1, 0, 0, 0, 0, 0, 0];
        let input_header = Header::try_from(&input_bytes[..]).unwrap();
        let mut output_bytes = ArrayBuffer::with_capacity(Header::SIZE);
        input_header.write_into(&mut output_bytes);
        assert_eq!(&input_bytes[..Header::SIZE], &output_bytes[..Header::SIZE]);
    }
}
