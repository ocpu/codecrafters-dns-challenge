mod header;
mod question;
mod types;
mod packet;

use std::{net::UdpSocket, sync::Arc};

use types::{QClass, QType};

use crate::{header::{Header, ResponseCode}, packet::DNSPacket};

const MAX_MESSAGE_SIZE: usize = 512;

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

        for q in packet.questions() {
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
