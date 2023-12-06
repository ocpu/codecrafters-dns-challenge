//! The DNS packet header is a struct with an id a few flags and the number of questions, answers,
//! name servers in the authority section, and the number of additional records. The structure is
//! as the following, and multibyte items are in big endian order. 
//! ```text
//!                                     1  1  1  1  1  1
//!       0  1  2  3  4  5  6  7  8  9  0  1  2  3  4  5
//!     +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
//!     |                      ID                       |
//!     +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
//!     |QR|   Opcode  |AA|TC|RD|RA|   Z    |   RCODE   |
//!     +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
//!     |                    QDCOUNT                    |
//!     +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
//!     |                    ANCOUNT                    |
//!     +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
//!     |                    NSCOUNT                    |
//!     +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
//!     |                    ARCOUNT                    |
//!     +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
//! ```
//!
//! - **ID**: This is the id of the packet. If the packet gets truncated, then this field groups
//!   more than 1 packet together.
//! - **QR**: This is the bit that defines if the packet is a query or response. Query is defined
//!   with a 0 and a response is defined by 1. They map to the [PacketType] enum.
//! - **Opcode**: This is a number that defines what type of query/response the packet is for. See
//!   [Opcode].
//! - **AA**: If the responding name server is an authority for the domain name, then this bit is
//!   set to indicate that.
//! - **TC**: If the query/response is truncated, then this bit is set. It would indicate that the
//!   query/response needs to be aggregated with other packets.
//! - **RD**: If the name server does not an authority for the domain name then recursivly pursue
//!   the query.
//! - **RA**: Set in responses to indicate that the server supports recursive queries.
//! - **Z**: Reserved bits for future use. Must be 0 in all queries and responses.
//! - **RCODE**: A response code only relevant when responding or reading a response. It can
//!   indicate various error contitions or success. Read the enum [ResponseCode] for a little more
//!   info on the various conditions.
//! - **QDCOUNT**: The amount of questions the DNS packet has.
//! - **ANCOUNT**: The amount of answers the DNS packet has.
//! - **NSCOUNT**: The amount of name servers present in the DNS packet authority section.
//! - **ARCOUNT**: The amount of additional records the DNS packet has.
//!
//! ## Getting DNS header information
//!
//! To default way to get information is from the [DNSHeaderView] struct. It does not do any
//! validation on initial creation as that is deferred to the retreival methods. If validation is
//! desired immediatly, then use [DNSHeaderView::new_validated] to get a validated view of the
//! header.
//!
//! ```
//! let header: [u8; 12] = [4, 210, 16, 0, 0, 1, 0, 0, 0, 0, 0, 0];
//!
//! let view = DNSHeaderView::new(&header);
//! assert_eq!(view.packet_type(), Some(PacketType::Response));
//! assert_eq!(view.opcode(), Ok(Some(Opcode::Query)));
//! assert_eq!(view.question_entries(), Some(1));
//! println!("{view:?}");
//!
//! let view = DNSHeaderView::new_validated(&header)
//!     .expect("Header values to be correct")
//!     .expect("Header to not be empty");
//! assert_eq!(view.packet_type(), PacketType::Response);
//! assert_eq!(view.opcode(), Opcode::Query);
//! assert_eq!(view.question_entries(), 1);
//! println!("{view:?}");
//! ```

use std::{fmt::Debug, marker::PhantomData};

use thiserror::Error;

use crate::header::{Opcode, PacketType, ResponseCode};

#[derive(Clone)]
struct DNSHeaderView_<'data, State>(&'data [u8], PhantomData<State>);
struct Invalid;
struct Valid;

pub type DNSHeaderViewValidated<'data> = DNSHeaderView_<'data, Valid>;
pub type DNSHeaderView<'data> = DNSHeaderView_<'data, Invalid>;

#[derive(Debug, Error)]
#[error("The header specified an unknown opcode: {0}")]
pub struct UnknownOpcodeError(u8);

#[derive(Debug, Error)]
#[error("The header specified an unknown response code: {0}")]
pub struct UnknownResponseCodeError(u8);

#[derive(Debug, Error)]
pub enum DNSHeaderViewError<'data> {
    #[error("The size of the header buffer was {0} expected 12")]
    IncorrectHeaderSize(usize),

    #[error("The DNS header specified incorrectly has reserved bits specified")]
    ReservedBitsInUse(DNSHeaderViewValidated<'data>),

    #[error(transparent)]
    UnknownOpcode(#[from] UnknownOpcodeError),

    #[error(transparent)]
    UnknownResponseCode(#[from] UnknownResponseCodeError),
}

impl<'data, State> DNSHeaderView_<'data, State> {
    pub const SIZE: usize = 12;
}

impl<'data> DNSHeaderView_<'data, Invalid> {
    pub const fn new(buffer: &'data [u8]) -> Self {
        Self(buffer, PhantomData)
    }

    pub const fn new_validated(
        buffer: &'data [u8],
    ) -> Result<Option<DNSHeaderView_<'data, Valid>>, DNSHeaderViewError<'data>> {
        DNSHeaderView_::<'data, Valid>::new(buffer)
    }

    pub const fn into_manually_validated(self) -> DNSHeaderView_<'data, Valid> {
        DNSHeaderView_(self.0, PhantomData)
    }

    /// A 16 bit identifier assigned by the program that
    /// generates any kind of query.  This identifier is copied
    /// the corresponding reply and can be used by the requester
    /// to match up replies to outstanding queries.
    ///
    /// Field: ID
    pub const fn id(&self) -> Option<u16> {
        if self.0.len() < 2 {
            return None;
        }
        Some(u16::from_be_bytes([self.0[0], self.0[1]]))
    }

    /// A one bit field that specifies whether this message is a
    /// query (0), or a response (1).
    ///
    /// Field: QR
    pub const fn packet_type(&self) -> Option<PacketType> {
        if self.0.len() < 3 {
            None
        } else if (self.0[2] & 0x80) == 0x80 {
            Some(PacketType::Response)
        } else {
            Some(PacketType::Query)
        }
    }

    /// A four bit field that specifies kind of query in this
    /// message.  This value is set by the originator of a query
    /// and copied into the response.
    ///
    /// Field: Opcode
    pub const fn opcode(&self) -> Result<Option<Opcode>, UnknownOpcodeError> {
        if self.0.len() < 3 {
            return Ok(None);
        }
        Ok(Some(match (self.0[2] >> 3) & 0xf {
            0 => Opcode::Query,
            1 => Opcode::InverseQuery,
            2 => Opcode::Status,
            code => return Err(UnknownOpcodeError(code)),
        }))
    }

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
    pub const fn authoritive_answer(&self) -> Option<bool> {
        if self.0.len() < 3 {
            None
        } else {
            Some((self.0[2] & 4) == 4)
        }
    }

    /// TrunCation - specifies that this message was truncated
    /// due to length greater than that permitted on the
    /// transmission channel.
    ///
    /// Field: TC
    pub const fn truncated(&self) -> Option<bool> {
        if self.0.len() < 3 {
            None
        } else {
            Some((self.0[2] & 2) == 2)
        }
    }

    /// Recursion Desired - this bit may be set in a query and
    /// is copied into the response.  If RD is set, it directs
    /// the name server to pursue the query recursively.
    /// Recursive query support is optional.
    ///
    /// Field: RD
    pub const fn recursion_desired(&self) -> Option<bool> {
        if self.0.len() < 3 {
            None
        } else {
            Some((self.0[2] & 1) == 1)
        }
    }

    /// Recursion Available - this be is set or cleared in a
    /// response, and denotes whether recursive query support is
    /// available in the name server.
    ///
    /// Field: RA
    pub const fn recursion_available(&self) -> Option<bool> {
        if self.0.len() < 4 {
            None
        } else {
            Some((self.0[3] & 0xf0) == 0xf0)
        }
    }

    /// Response code - this 4 bit field is set as part of responses.
    ///
    /// Field: RCODE
    pub const fn response_code(&self) -> Result<Option<ResponseCode>, UnknownResponseCodeError> {
        if self.0.len() < 4 {
            return Ok(None);
        }
        Ok(Some(match self.0[3] & 0xf {
            0 => ResponseCode::None,
            1 => ResponseCode::FormatError,
            2 => ResponseCode::ServerFailure,
            3 => ResponseCode::NameError,
            4 => ResponseCode::NotImplemented,
            5 => ResponseCode::Refused,
            code => return Err(UnknownResponseCodeError(code)),
        }))
    }

    /// An unsigned 16 bit integer specifying the number of
    /// entries in the question section.
    ///
    /// Field: QDCOUNT
    pub const fn question_entries(&self) -> Option<u16> {
        if self.0.len() < 6 {
            return None;
        }
        Some(u16::from_be_bytes([self.0[4], self.0[5]]))
    }

    /// An unsigned 16 bit integer specifying the number of
    /// resource records in the answer section.
    ///
    /// Field: ANCOUNT
    pub const fn answer_entries(&self) -> Option<u16> {
        if self.0.len() < 8 {
            return None;
        }
        Some(u16::from_be_bytes([self.0[6], self.0[7]]))
    }

    /// An unsigned 16 bit integer specifying the number of name
    /// server resource records in the authority records
    /// section.
    ///
    /// Field: NSCOUNT
    pub const fn authority_entries(&self) -> Option<u16> {
        if self.0.len() < 10 {
            return None;
        }
        Some(u16::from_be_bytes([self.0[8], self.0[9]]))
    }

    /// An unsigned 16 bit integer specifying the number of
    /// resource records in the additional records section.
    ///
    /// Field: ARCOUNT
    pub const fn additional_entries(&self) -> Option<u16> {
        if self.0.len() < 12 {
            return None;
        }
        Some(u16::from_be_bytes([self.0[10], self.0[11]]))
    }
}

impl<'data> DNSHeaderView_<'data, Valid> {
    pub const fn new(buffer: &'data [u8]) -> Result<Option<Self>, DNSHeaderViewError<'data>> {
        if buffer.is_empty() {
            return Ok(None);
        }
        if buffer.len() != Self::SIZE {
            return Err(DNSHeaderViewError::IncorrectHeaderSize(buffer.len()));
        }
        match (buffer[2] >> 3) & 0xf {
            0..=2 => {}
            code => return Err(DNSHeaderViewError::UnknownOpcode(UnknownOpcodeError(code))),
        }
        match buffer[3] & 0xf {
            0..=5 => {}
            code => {
                return Err(DNSHeaderViewError::UnknownResponseCode(
                    UnknownResponseCodeError(code),
                ))
            }
        }
        if (buffer[3] & 0b1110) != 0 {
            return Err(DNSHeaderViewError::ReservedBitsInUse(Self(
                buffer,
                PhantomData,
            )));
        }
        Ok(Some(Self(buffer, PhantomData)))
    }

    /// A 16 bit identifier assigned by the program that
    /// generates any kind of query.  This identifier is copied
    /// the corresponding reply and can be used by the requester
    /// to match up replies to outstanding queries.
    ///
    /// Field: ID
    pub const fn id(&self) -> u16 {
        u16::from_be_bytes([self.0[0], self.0[1]])
    }

    /// A one bit field that specifies whether this message is a
    /// query (0), or a response (1).
    ///
    /// Field: QR
    pub const fn packet_type(&self) -> PacketType {
        if (self.0[2] & 0x80) == 0x80 {
            PacketType::Response
        } else {
            PacketType::Query
        }
    }

    /// A four bit field that specifies kind of query in this
    /// message.  This value is set by the originator of a query
    /// and copied into the response.
    ///
    /// Field: Opcode
    pub const fn opcode(&self) -> Opcode {
        match (self.0[2] >> 3) & 0xf {
            0 => Opcode::Query,
            1 => Opcode::InverseQuery,
            2 => Opcode::Status,
            _ => panic!("Opcode should already be checked!"),
        }
    }

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
    pub const fn authoritive_answer(&self) -> bool {
        (self.0[2] & 4) == 4
    }

    /// TrunCation - specifies that this message was truncated
    /// due to length greater than that permitted on the
    /// transmission channel.
    ///
    /// Field: TC
    pub const fn truncated(&self) -> bool {
        (self.0[2] & 2) == 2
    }

    /// Recursion Desired - this bit may be set in a query and
    /// is copied into the response.  If RD is set, it directs
    /// the name server to pursue the query recursively.
    /// Recursive query support is optional.
    ///
    /// Field: RD
    pub const fn recursion_desired(&self) -> bool {
        (self.0[2] & 1) == 1
    }

    /// Recursion Available - this be is set or cleared in a
    /// response, and denotes whether recursive query support is
    /// available in the name server.
    ///
    /// Field: RA
    pub const fn recursion_available(&self) -> bool {
        (self.0[3] & 0xf0) == 0xf0
    }

    /// Response code - this 4 bit field is set as part of responses.
    ///
    /// Field: RCODE
    pub const fn response_code(&self) -> ResponseCode {
        match self.0[3] & 0xf {
            0 => ResponseCode::None,
            1 => ResponseCode::FormatError,
            2 => ResponseCode::ServerFailure,
            3 => ResponseCode::NameError,
            4 => ResponseCode::NotImplemented,
            5 => ResponseCode::Refused,
            _ => panic!("Response code should already be checked"),
        }
    }

    /// An unsigned 16 bit integer specifying the number of
    /// entries in the question section.
    ///
    /// Field: QDCOUNT
    pub const fn question_entries(&self) -> u16 {
        u16::from_be_bytes([self.0[4], self.0[5]])
    }

    /// An unsigned 16 bit integer specifying the number of
    /// resource records in the answer section.
    ///
    /// Field: ANCOUNT
    pub const fn answer_entries(&self) -> u16 {
        u16::from_be_bytes([self.0[6], self.0[7]])
    }

    /// An unsigned 16 bit integer specifying the number of name
    /// server resource records in the authority records
    /// section.
    ///
    /// Field: NSCOUNT
    pub const fn authority_entries(&self) -> u16 {
        u16::from_be_bytes([self.0[8], self.0[9]])
    }

    /// An unsigned 16 bit integer specifying the number of
    /// resource records in the additional records section.
    ///
    /// Field: ARCOUNT
    pub const fn additional_entries(&self) -> u16 {
        u16::from_be_bytes([self.0[10], self.0[11]])
    }
}

impl<'data> Debug for DNSHeaderView_<'data, Invalid> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut ds = f.debug_struct("DNSHeaderView");
        if let Some(val) = self.id() {
            ds.field("id", &val);
        } else {
            return ds.finish_non_exhaustive();
        }
        if let Some(val) = self.packet_type() {
            ds.field("packet_type", &val);
        } else {
            return ds.finish_non_exhaustive();
        }
        let _ = match self.opcode() {
            Ok(Some(val)) => ds.field("opcode", &val),
            Ok(None) => return ds.finish_non_exhaustive(),
            Err(err) => ds.field("opcode", &err),
        };
        if let Some(val) = self.authoritive_answer() {
            ds.field("authoritive_answer", &val);
        } else {
            return ds.finish_non_exhaustive();
        }
        if let Some(val) = self.truncated() {
            ds.field("truncated", &val);
        } else {
            return ds.finish_non_exhaustive();
        }
        if let Some(val) = self.recursion_desired() {
            ds.field("recursion_desired", &val);
        } else {
            return ds.finish_non_exhaustive();
        }
        if let Some(val) = self.recursion_available() {
            ds.field("recursion_available", &val);
        } else {
            return ds.finish_non_exhaustive();
        }
        let _ = match self.response_code() {
            Ok(Some(val)) => ds.field("response_code", &val),
            Ok(None) => return ds.finish_non_exhaustive(),
            Err(err) => ds.field("response_code", &err),
        };
        if let Some(val) = self.question_entries() {
            ds.field("question_entries", &val);
        } else {
            return ds.finish_non_exhaustive();
        }
        if let Some(val) = self.answer_entries() {
            ds.field("answer_entries", &val);
        } else {
            return ds.finish_non_exhaustive();
        }
        if let Some(val) = self.authority_entries() {
            ds.field("authority_entries", &val);
        } else {
            return ds.finish_non_exhaustive();
        }
        if let Some(val) = self.additional_entries() {
            ds.field("additional_entries", &val);
        } else {
            return ds.finish_non_exhaustive();
        }
        ds.finish()
    }
}

impl<'data> Debug for DNSHeaderView_<'data, Valid> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DNSHeaderView")
            .field("id", &self.id())
            .field("packet_type", &self.packet_type())
            .field("opcode", &self.opcode())
            .field("authoritive_answer", &self.authoritive_answer())
            .field("truncated", &self.truncated())
            .field("recursion_desired", &self.recursion_desired())
            .field("recursion_available", &self.recursion_available())
            .field("response_code", &self.response_code())
            .field("question_entries", &self.question_entries())
            .field("answer_entries", &self.answer_entries())
            .field("authority_entries", &self.authority_entries())
            .field("additional_entries", &self.additional_entries())
            .finish()
    }
}
