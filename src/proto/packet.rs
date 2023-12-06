use thiserror::Error;

use std::fmt;

use super::{
    DebugList, FromPacketBytes, HeaderViewError, HeaderViewValidated, Question, QuestionError,
    Resource, ResourceError,
};

pub struct Packet<'data> {
    header: HeaderViewValidated<'data>,
    first_question: Option<Question<'data>>,
    first_answer: Option<Resource<'data>>,
    first_autoritive: Option<Resource<'data>>,
    first_additional: Option<Resource<'data>>,
}

#[derive(Debug, Error)]
pub enum PacketError {
    #[error(transparent)]
    Header(#[from] HeaderViewError),
    #[error("Expected to find {expected} questions but found {found} instead")]
    TooFewQuestions { expected: usize, found: usize },
    #[error("No questions found when validating packet")]
    NoQuestions,
    #[error(transparent)]
    Question(#[from] QuestionError),
    #[error("Expected to find {expected} answers but found {found} instead")]
    TooFewAnswers { expected: usize, found: usize },
    #[error("No answers found when validating packet")]
    NoAnswers,
    #[error(transparent)]
    Answer(ResourceError),
    #[error("Expected to find {expected} authority items but found {found} instead")]
    TooFewAuthoriryItems { expected: usize, found: usize },
    #[error("No authority items found when validating packet")]
    NoAuthorityItems,
    #[error(transparent)]
    AuthoritiveItem(ResourceError),
    #[error("Expected to find {expected} additional items but found {found} instead")]
    TooFewAdditionalItems { expected: usize, found: usize },
    #[error("No additional items found when validating packet")]
    NoAdditionalItems,
    #[error(transparent)]
    AdditionalItem(ResourceError),
    #[error("TODO")]
    EOF,
}

struct QuestionIter<'data>(usize, Option<Question<'data>>);
struct ResourceIter<'data>(usize, Option<Resource<'data>>);

impl<'data> Packet<'data> {
    pub fn header(&self) -> &HeaderViewValidated<'data> {
        &self.header
    }

    pub fn questions(&self) -> impl Iterator<Item = Question<'data>> {
        QuestionIter(self.header.question_entries() as usize, self.first_question)
    }

    pub fn answers(&self) -> impl Iterator<Item = Resource<'data>> {
        ResourceIter(self.header.answer_entries() as usize, self.first_answer)
    }

    pub fn authority(&self) -> impl Iterator<Item = Resource<'data>> {
        ResourceIter(
            self.header.authority_entries() as usize,
            self.first_autoritive,
        )
    }

    pub fn additional(&self) -> impl Iterator<Item = Resource<'data>> {
        ResourceIter(
            self.header.additional_entries() as usize,
            self.first_additional,
        )
    }
}

impl<'data> FromPacketBytes<'data> for Packet<'data> {
    type Error = PacketError;

    fn parse(bytes: &'data [u8], offset: usize) -> Result<Option<Self>, Self::Error> {
        let Some(header) = HeaderViewValidated::parse(bytes, offset)? else {
            return Ok(None);
        };

        let mut packet_offset = offset + HeaderViewValidated::SIZE;
        let mut first_question: Option<Question<'data>> = None;
        let mut first_answer: Option<Resource<'data>> = None;
        let mut first_autoritive: Option<Resource<'data>> = None;
        let mut first_additional: Option<Resource<'data>> = None;

        let mut questions = header.question_entries() as usize;
        let mut answers = header.answer_entries() as usize;
        let mut authoritive_items = header.authority_entries() as usize;
        let mut additional_items = header.additional_entries() as usize;

        // TODO: Handle abuse of incoming entry counts

        while packet_offset < bytes.len() {
            if questions > 0 {
                let Some(question) = Question::parse(bytes, packet_offset)? else {
                    if first_question.is_none() {
                        return Err(PacketError::NoQuestions);
                    } else {
                        return Err(PacketError::TooFewQuestions {
                            expected: header.question_entries() as usize,
                            found: (header.question_entries() as usize) - questions,
                        });
                    }
                };
                if first_question.is_none() {
                    first_question = Some(question);
                }
                questions -= 1;
                packet_offset += question.size_in_packet();
                continue;
            }
            if answers > 0 {
                let Some(answer) =
                    Resource::parse(bytes, packet_offset).map_err(PacketError::Answer)?
                else {
                    if first_answer.is_none() {
                        return Err(PacketError::NoAnswers);
                    } else {
                        return Err(PacketError::TooFewAnswers {
                            expected: header.answer_entries() as usize,
                            found: (header.answer_entries() as usize) - answers,
                        });
                    }
                };
                if first_answer.is_none() {
                    first_answer = Some(answer);
                }
                answers -= 1;
                packet_offset += answer.size_in_packet();
                continue;
            }
            if authoritive_items > 0 {
                let Some(authoritive_item) =
                    Resource::parse(bytes, packet_offset).map_err(PacketError::AuthoritiveItem)?
                else {
                    if first_autoritive.is_none() {
                        return Err(PacketError::NoAuthorityItems);
                    } else {
                        return Err(PacketError::TooFewAuthoriryItems {
                            expected: header.authority_entries() as usize,
                            found: (header.authority_entries() as usize) - authoritive_items,
                        });
                    }
                };
                if first_autoritive.is_none() {
                    first_autoritive = Some(authoritive_item);
                }
                authoritive_items -= 1;
                packet_offset += authoritive_item.size_in_packet();
                continue;
            }
            if additional_items > 0 {
                let Some(additional_item) =
                    Resource::parse(bytes, packet_offset).map_err(PacketError::AdditionalItem)?
                else {
                    if first_additional.is_none() {
                        return Err(PacketError::NoAdditionalItems);
                    } else {
                        return Err(PacketError::TooFewAdditionalItems {
                            expected: header.additional_entries() as usize,
                            found: (header.additional_entries() as usize) - additional_items,
                        });
                    }
                };
                if first_additional.is_none() {
                    first_additional = Some(additional_item);
                }
                additional_items -= 1;
                packet_offset += additional_item.size_in_packet();
                continue;
            }
            break;
        }
        if packet_offset > bytes.len() {
            return Err(PacketError::EOF);
        } else if questions > 0 {
            if first_question.is_none() {
                return Err(PacketError::NoQuestions);
            } else {
                return Err(PacketError::TooFewQuestions {
                    expected: header.question_entries() as usize,
                    found: (header.question_entries() as usize) - questions,
                });
            }
        } else if answers > 0 {
            if first_answer.is_none() {
                return Err(PacketError::NoAnswers);
            } else {
                return Err(PacketError::TooFewAnswers {
                    expected: header.answer_entries() as usize,
                    found: (header.answer_entries() as usize) - answers,
                });
            }
        } else if authoritive_items > 0 {
            if first_autoritive.is_none() {
                return Err(PacketError::NoAuthorityItems);
            } else {
                return Err(PacketError::TooFewAuthoriryItems {
                    expected: header.authority_entries() as usize,
                    found: (header.authority_entries() as usize) - authoritive_items,
                });
            }
        } else if additional_items > 0 {
            if first_additional.is_none() {
                return Err(PacketError::NoAdditionalItems);
            } else {
                return Err(PacketError::TooFewAdditionalItems {
                    expected: header.additional_entries() as usize,
                    found: (header.additional_entries() as usize) - additional_items,
                });
            }
        } else {
            Ok(Some(Self {
                header,
                first_question,
                first_answer,
                first_autoritive,
                first_additional,
            }))
        }
    }
}

impl<'data> Iterator for QuestionIter<'data> {
    type Item = Question<'data>;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(item) = self.1 else {
            return None;
        };
        if self.0 <= 1 {
            self.0 = 0;
            self.1 = None;
        } else {
            self.0 -= 1;
            // Unwrap once to remove the Result as it has already been checked in parsing the
            // buffer.
            self.1 = Question::parse(item.buffer, item.offset + item.size_in_packet()).unwrap();
        }
        Some(item)
    }
}

impl<'data> Iterator for ResourceIter<'data> {
    type Item = Resource<'data>;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(item) = self.1 else {
            return None;
        };
        if self.0 <= 1 {
            self.0 = 0;
            self.1 = None;
        } else {
            self.0 -= 1;
            // Unwrap once to remove the Result as it has already been checked in parsing the
            // buffer.
            self.1 = Resource::parse(item.buffer, item.offset + item.size_in_packet()).unwrap();
        }
        Some(item)
    }
}

impl<'data> fmt::Debug for Packet<'data> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("Packet");
        s.field("header", &self.header);
        s.field("questions", &DebugList(|| self.questions()));
        s.field("answers", &DebugList(|| self.answers()));
        s.field("authority", &DebugList(|| self.authority()));
        s.field("additional", &DebugList(|| self.additional()));
        s.finish()
    }
}
