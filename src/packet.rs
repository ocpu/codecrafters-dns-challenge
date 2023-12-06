use std::{collections::hash_map::DefaultHasher, hash::Hasher};

use bytes::BufMut;

use crate::{
    array_buffer::ArrayBuffer,
    domain_name::DomainName,
    header::Header,
    proto::{HeaderView, Opcode, PacketType, ResponseCode},
    question::Question,
    resource::Resource,
};

pub struct DNSPacketBuilder {
    header: Header,
    questions: Vec<Question>,
    answers: Vec<Resource>,
    compress: bool,
}

impl DNSPacketBuilder {
    pub fn respond<'data>(packet: &crate::proto::Packet<'data>, code: ResponseCode) -> Self {
        let mut header = Header::new(packet.header().id());
        header.opcode = packet.header().opcode();
        header.recursion_desired = packet.header().recursion_desired();
        header.packet_type = PacketType::Response;
        header.response_code = code;

        Self {
            header,
            questions: Vec::new(),
            answers: Vec::new(),
            compress: true,
        }
    }

    pub fn respond_to(header: HeaderView, code: ResponseCode) -> Self {
        let mut h = Header::new(header.id().unwrap_or_default());
        h.opcode = Opcode::Query;
        h.recursion_desired = header.recursion_desired().unwrap_or_default();
        h.packet_type = PacketType::Response;
        h.response_code = code;

        Self {
            header: h,
            questions: Vec::new(),
            answers: Vec::new(),
            compress: true,
        }
    }

    pub fn query(id: u16) -> Self {
        let mut header = Header::new(id);
        header.opcode = Opcode::Query;
        header.recursion_desired = true;
        header.packet_type = PacketType::Query;

        DNSPacketBuilder {
            header,
            compress: true,
            questions: Vec::new(),
            answers: Vec::new(),
        }
    }

    pub fn add_question(mut self, question: Question) -> Self {
        self.questions.push(question);
        self.header.question_entries += 1;
        self
    }

    pub fn add_answer(mut self, answer: Resource) -> Self {
        self.answers.push(answer);
        self.header.answer_entries += 1;
        self
    }

    pub fn build_into<'a>(self, buffer: &'a mut ArrayBuffer) {
        self.header.write_into(buffer);

        let mut written_names: Vec<(u64, usize)> = Vec::new();
        //let mut truncate = false;

        for question in self.questions {
            let start = buffer.len();
            match write_name(buffer, question.name(), self.compress, &mut written_names) {
                Ok(()) => {}
                Err(TooLong) => {
                    set_truncated(buffer, start);
                    //truncate = true;
                    break;
                }
            };

            if buffer.remaining_mut() < 4 {
                set_truncated(buffer, start);
                //truncate = true;
                break;
            }

            buffer.put_u16(question.q_type().as_u16());
            buffer.put_u16(question.q_class().as_u16());
        }

        /*truncate = truncate || */
        write_resource_list(
            buffer,
            self.answers.into_iter(),
            self.compress,
            &mut written_names,
        );
    }
}

fn set_truncated(buffer: &mut ArrayBuffer, new_len: usize) {
    buffer.set_len(new_len);
    buffer.as_slice_mut()[2] |= 2;
}

fn write_resource_list(
    buffer: &mut ArrayBuffer,
    iter: impl Iterator<Item = Resource>,
    compress: bool,
    written_names: &mut Vec<(u64, usize)>,
) -> bool {
    for Resource(name, data) in iter {
        let start = buffer.len();

        match write_name(buffer, &name, compress, written_names) {
            Ok(()) => {}
            Err(TooLong) => {
                set_truncated(buffer, start);
                return true;
            }
        };

        let dat = data.data();
        if buffer.remaining_mut() < 10 + dat.len() {
            set_truncated(buffer, start);
            return true;
        }

        buffer.put_u16(data.typ().as_u16());
        buffer.put_u16(data.class().as_u16());
        buffer.put_u32(*data.ttl());
        buffer.put_u16(dat.len() as u16);
        buffer.put_slice(dat.as_ref());
    }

    return false;
}

struct TooLong;

fn write_name(
    buffer: &mut ArrayBuffer,
    domain_name: &DomainName,
    compress: bool,
    written_names: &mut Vec<(u64, usize)>,
) -> Result<(), TooLong> {
    use std::hash::Hash;
    let mut hasher = DefaultHasher::default();
    domain_name.hash(&mut hasher);
    let hash = hasher.finish();

    if compress
        && written_names
            .iter()
            .any(|(name_hash, _)| *name_hash == hash)
    {
        if buffer.remaining_mut() < 2 {
            return Err(TooLong);
        }
        let (_, offset) = written_names
            .iter()
            .find(|(name_hash, _)| *name_hash == hash)
            .unwrap();
        buffer.put_u8((((*offset >> 8) as u8) & 0x3f) | 0xc0);
        buffer.put_u8(*offset as u8);
        Ok(())
    } else {
        let mut pointer = 0;
        for (index, label) in domain_name.labels().enumerate() {
            if buffer.remaining_mut() < 1 + label.len() {
                return Err(TooLong);
            }
            buffer.put_u8(label.len() as u8);
            buffer.put_slice(label.as_bytes());

            if index + 1 == domain_name.len() {
                break;
            }
            if compress {
                let next_hash = {
                    let mut hasher = DefaultHasher::default();
                    domain_name
                        .labels()
                        .skip(index)
                        .for_each(|label| label.hash(&mut hasher));
                    hasher.finish()
                };

                if let Some((_, offset)) = written_names
                    .iter()
                    .find(|(name_hash, _)| *name_hash == next_hash)
                {
                    pointer = domain_name.len() - index;
                    if buffer.remaining_mut() < 2 {
                        return Err(TooLong);
                    }
                    buffer.put_u8((((*offset >> 8) as u8) & 0x3f) | 0xc0);
                    buffer.put_u8(*offset as u8);
                    break;
                }
            }
        }
        if pointer == 0 {
            if buffer.remaining_mut() < 1 {
                return Err(TooLong);
            }
            buffer.put_u8(0);
        }

        // 1 => 1..=1 = 1     | [1 - 1..]
        // 2 => 1..=2 = 1, 2  | [2 - 1..], [2 - 2..]
        for i in 1..=domain_name.len() - pointer {
            let labels: Vec<_> = domain_name.labels().skip(domain_name.len() - i).collect();

            // The 1 here refers to the last "null" in the name sequence.
            // While the 1 in the map refers to the length byte.
            let buffer_len: usize = 1 + labels.iter().map(|label| label.len() + 1).sum::<usize>();
            let hash = {
                let mut hasher = DefaultHasher::default();
                labels.iter().for_each(|label| label.hash(&mut hasher));
                hasher.finish()
            };
            written_names.push((hash, buffer.len() - buffer_len))
        }

        Ok(())
    }
}
