mod domain_name;
mod header;
mod label;
mod packet;
mod question;
mod resource;
mod types;

use tracing::Level;

use crate::{
    domain_name::DomainName,
    header::{Opcode, ResponseCode},
    packet::DNSPacket,
    question::Question,
    resource::ARecord,
};
use std::{
    collections::HashMap,
    net::{Ipv4Addr, UdpSocket},
};

const MAX_MESSAGE_SIZE: usize = 512;

fn main() {
    tracing_subscriber::fmt().with_max_level(Level::TRACE).init();
    let mut map = HashMap::new();
    map.insert(
        DomainName::from_static("codecrafters.io"),
        ARecord::new(500, Ipv4Addr::new(8, 8, 8, 8)),
    );

    let udp_socket = UdpSocket::bind("127.0.0.1:2053").expect("Failed to bind to address");
    let mut buf = [0; MAX_MESSAGE_SIZE];
    let mut response = [0; MAX_MESSAGE_SIZE];

    loop {
        let Ok((size, source)) = udp_socket.recv_from(&mut buf) else {
            eprintln!("ERROR: receiving data from socket");
            break;
        };
        let span = tracing::trace_span!("dns_request", source = %source);
        let _guard = span.enter();
//        if cfg!(debug_assertions) {
            print_buffer("Input", &buf[..size]);
//        }
        let packet = match DNSPacket::try_parse(&buf[..size]) {
            Ok(packet) => packet,
            Err(e) => {
                tracing::error!(error = "Failed to parse packet", message = ?e);
                let (_, resp_size) = DNSPacket::try_parse_header_only(&buf[..size])
                    .unwrap_or_else(|| DNSPacket::new(0))
                    .respond(ResponseCode::FormatError)
                    .build_into(&mut response[..])
                    .expect("Packet header is too large for buffer");

                let Ok(_) = udp_socket.send_to(&response[..resp_size], source) else {
                    eprintln!("Failed to send response");
                    continue;
                };
                continue;
            }
        };

        let mut builder = packet.respond(match packet.header().opcode {
            Opcode::Query => ResponseCode::None,
            _ => ResponseCode::NotImplemented,
        });

        for q in packet.questions() {
            tracing::trace!(section = "question", domain_name = %q.name(), r#type = ?q.q_type(), class = ?q.q_class());
            builder =
                builder.add_question(Question::new(*q.q_type(), *q.q_class(), q.name().clone()));
            builder = builder.add_answer(
                ARecord::new(500, Ipv4Addr::new(8, 8, 8, 8)).to_resource(q.name().clone()),
            );
            /*builder = match map.get(q.name()) {
                Some(record) => builder.add_answer(record.to_resource(q.name().clone())),
                None => builder,
            }*/
        }

        let (_response_header, resp_size) = builder
            .disable_compression()
            .build_into(&mut response[..])
            .expect("TODO: split response?");

//        if cfg!(debug_assertions) {
            print_buffer("Output", &response[..resp_size]);
//        }
        udp_socket
            .send_to(&response[..resp_size], source)
            .expect("Failed to send response");
    }
}

//#[cfg(debug_assertions)]
const LINE_ITEM_COUNT: usize = 16;

//#[cfg(debug_assertions)]
fn print_buffer(label: &str, mut buffer: &[u8]) {
    use nom::AsChar;

    eprintln!("--- Begin {label} ---");
    loop {
        let slice = &buffer[..usize::min(LINE_ITEM_COUNT, buffer.len())];
        buffer = &buffer[slice.len()..];

        for i in 0..LINE_ITEM_COUNT {
            if let Some(byte) = slice.get(i) {
                if *byte < 16 {
                    eprint!("0");
                }
                eprint!("{byte:x?} ");
            } else {
                eprint!("   ");
            }
        }

        for i in 0..LINE_ITEM_COUNT {
            if let Some(byte) = slice.get(i) {
                if byte.is_ascii_alphanumeric() || *byte == b'-' {
                    eprint!("{}", byte.as_char());
                } else {
                    eprint!(".");
                }
            } else {
                eprint!(" ");
            }
        }
        eprint!("\n");
        if buffer.is_empty() {
            break;
        }
    }
    eprintln!("--- End {label} ---");
}
