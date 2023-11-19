mod header;
mod packet;
mod question;
mod resource;
mod types;

use crate::{
    header::ResponseCode,
    packet::DNSPacket,
    resource::ARecord,
    types::{DomainName, DomainNameOwned, QType, QClass}, question::Question,
};
use std::{net::UdpSocket, sync::Arc};

const MAX_MESSAGE_SIZE: usize = 512;

fn main() {
    println!("Logs from your program will appear here!");

    let udp_socket = UdpSocket::bind("127.0.0.1:2053").expect("Failed to bind to address");
    let mut buf = [0; MAX_MESSAGE_SIZE];
    let mut response = [0; MAX_MESSAGE_SIZE];

    let domain_name: Arc<DomainNameOwned> = Arc::new(
        DomainName::from_str("codecrafters.io")
            .expect("'codecrafters.io' could not get turned into a DomainName")
            .into(),
    );

    loop {
        let Ok((size, source)) = udp_socket.recv_from(&mut buf) else {
            eprintln!("ERROR: receiving data from socket");
            break;
        };
        println!("Input: {:?}", &buf[..size]);
        let Ok(packet) = DNSPacket::try_parse(&buf[..size]) else {
            let id = if size >= 2 {
                u16::from_be_bytes([buf[0], buf[1]])
            } else {
                0
            };
            let (_, resp_size) = DNSPacket::builder(id)
                .response(ResponseCode::FormatError)
                .build_into(&mut response[..])
                .expect("Packet header is too large for buffer");
            let Ok(_) = udp_socket.send_to(&response[..resp_size], source) else {
                eprintln!("Failed to send response");
                continue;
            };
            continue;
        };

        for q in packet.questions() {
            println!("q = {q}");
        }

        let (_response_header, resp_size) = packet
            .respond_ok()
            .add_question(Question::new(QType::A, QClass::IN, (&domain_name).into()))
            .add_answer(ARecord::new((&domain_name).into(), 100, "8.8.8.8".parse().unwrap()).into())
            .build_into(&mut response[..])
            .expect("TODO: split response?");

        println!("Output: {:?}", &response[..resp_size]);
        udp_socket
            .send_to(&response[..resp_size], source)
            .expect("Failed to send response");
    }
}
