mod header;

use std::net::UdpSocket;

use crate::header::{Header, MessageType};

const _MAX_LABEL_SIZE: usize = 63;
const _MAX_NAME_SIZE: usize = 255;
const MAX_MESSAGE_SIZE: usize = 512;

fn main() {
    println!("Logs from your program will appear here!");

    let udp_socket = UdpSocket::bind("127.0.0.1:2053").expect("Failed to bind to address");
    let mut buf = [0; MAX_MESSAGE_SIZE];

    loop {
        match udp_socket.recv_from(&mut buf) {
            Ok((_size, source)) => {
                let header = match Header::try_from(&buf[..]) {
                    Ok(val) => val,
                    Err(_) => continue,
                };
                let mut response = [0; MAX_MESSAGE_SIZE];
                let mut response_header = Header::new(header.id);
                response_header.message_type = MessageType::Response;
                response_header.write_into(&mut response[..]);
                //println!("{:?}", &response[..Header::SIZE]);
                udp_socket
                    .send_to(&response[..Header::SIZE], source)
                    .expect("Failed to send response");
            }
            Err(e) => {
                eprintln!("Error receiving data: {}", e);
                break;
            }
        }
    }
}
