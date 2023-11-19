mod header;
mod packet;
mod question;
mod resource;
mod types;

use crate::{
    header::{Opcode, ResponseCode},
    packet::DNSPacket,
    question::Question,
    resource::ARecord,
    types::{CowDomainName, DomainName},
};
use std::{
    collections::HashMap,
    net::{Ipv4Addr, UdpSocket},
};

const MAX_MESSAGE_SIZE: usize = 512;

fn main() {
    println!("Logs from your program will appear here!");
    let mut map = HashMap::new();
    map.insert(
        CowDomainName::Borrowed(DomainName::from_str("codecrafters.io").unwrap()),
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
        println!("Input: {:?}", &buf[..size]);
        let packet = match DNSPacket::try_parse(&buf[..size]) {
            Ok(packet) => packet,
            Err(e) => {
                eprintln!("Failed to parse packet: {e:?}");
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
            println!("Question: {q}");
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
            .build_into(&mut response[..])
            .expect("TODO: split response?");

        println!("Output: {:?}", &response[..resp_size]);
        udp_socket
            .send_to(&response[..resp_size], source)
            .expect("Failed to send response");
    }
}
