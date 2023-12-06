mod array_buffer;
mod domain_name;
mod header;
mod label;
mod packet;
mod proto;
mod question;
mod resource;
mod types;

use array_buffer::ArrayBuffer;
use clap::Parser;
use packet::DNSPacketBuilder;
use proto::{FromPacketBytes, Opcode};
use thiserror::Error;
use tracing::{Instrument, Level};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    domain_name::DomainName,
    proto::ResponseCode,
    question::Question,
    resource::{Resource, ResourceData},
};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The resolver to use
    #[arg(short, long, default_value_t = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 53))]
    resolver: SocketAddr,

    /// More output
    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    /// Debug output
    #[arg[long, default_value_t = false]]
    vvv: bool,

    /// The port to listen on
    #[arg(short, long, default_value_t = 53)]
    port: u16,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = Args::parse();
    let max_level = if args.vvv {
        Level::DEBUG
    } else if args.verbose {
        Level::INFO
    } else {
        Level::WARN
    };

    tracing_subscriber::registry()
        //        .with(console_subscriber::spawn())
        .with(tracing_subscriber::filter::LevelFilter::from_level(
            max_level,
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
    let mut map = HashMap::new();
    map.insert(
        DomainName::from_static("codecrafters.io"),
        vec![
            ResourceData::A {
                ttl: 500,
                addr: [8, 8, 8, 8].into(),
            },
            ResourceData::A {
                ttl: 540,
                addr: Ipv4Addr::new(8, 8, 4, 4),
            },
        ],
    );

    let (tx, mut rx) = tokio::sync::mpsc::channel(1000);
    let udp_socket = Arc::new(
        tokio::net::UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], args.port)))
            .await
            .expect("Failed to bind to address"),
    );
    let forwarding_socket = tokio::net::UdpSocket::bind("0.0.0.0:0")
        .await
        .expect("Failed to bind to address");
    forwarding_socket
        .connect(args.resolver)
        .await
        .expect("Failed to connect to resolver");

    {
        let udp_socket = udp_socket.clone();
        tokio::spawn(async move {
            let mut response = ArrayBuffer::new().with_max_len(512);
            while let Some((buf, source)) = rx.recv().await {
                response.clear();
                async {
                    handle_dns_packet(buf, &mut response, &forwarding_socket, &map).await;
                    if response.len() > 0 {
                        if let Err(_) = udp_socket.send_to(response.as_slice(), source).await {
                            tracing::error!("Failed to send back to source");
                        }
                    }
                }
                .instrument(tracing::info_span!("dns_request", source = %source))
                .await
            }
        });
    }

    let mut buf = [0; 1024];

    tracing::info!(transport = "UDP", port = args.port, "Listening");

    loop {
        let Ok((size, source)) = udp_socket.recv_from(&mut buf).await else {
            eprintln!("ERROR: receiving data from socket");
            break;
        };

        if tx.send(((&buf[..size]).into(), source)).await.is_err() {
            break;
        }
    }
}

async fn handle_dns_packet(
    buf: ArrayBuffer,
    response: &mut ArrayBuffer,
    forwarding_socket: &tokio::net::UdpSocket,
    cache: &HashMap<DomainName, Vec<ResourceData>>,
) {
    if cfg!(debug_assertions) {
        //print_buffer("Input", &buf);
    }

    let packet = match proto::Packet::parse(&buf, 0) {
        Ok(Some(packet)) => packet,
        Ok(None) => return,
        Err(e) => {
            tracing::error!(error = "Failed to parse packet", message = ?e);
            response.clear();
            DNSPacketBuilder::respond_to(
                proto::HeaderView::new(&buf[..]),
                ResponseCode::FormatError,
            )
            .build_into(response);

            return;
        }
    };

    match packet.header().opcode() {
        Opcode::Query => {
            let mut builder = DNSPacketBuilder::respond(&packet, ResponseCode::None);
            let mut unknown_questions = Vec::new();
            for q in packet.questions() {
                tracing::debug!(section = "question", domain_name = %q.name(), r#type = ?q.q_type(), class = ?q.q_class());
                let name = (&q.name()).into();
                match cache.get(&name) {
                    Some(records) => {
                        builder = records.iter().fold(
                            builder.add_question(Question::new(
                                q.q_type().clone(),
                                q.q_class().clone(),
                                name.clone(),
                            )),
                            |b, record| b.add_answer(Resource(name.clone(), record.clone())),
                        )
                    }
                    None => unknown_questions.push(q),
                }
            }
            if !unknown_questions.is_empty() {
                builder =
                    match forward_request(&forwarding_socket, &packet, &unknown_questions, builder)
                        .await
                    {
                        Ok(b) => b,
                        Err(e) => {
                            tracing::error!(error = "Failed to parse packet", message = ?e);
                            DNSPacketBuilder::respond(
                                &packet,
                                match e {
                                    ForwardError::IO(_) => ResponseCode::Refused,
                                    ForwardError::ParsePacket(_) => ResponseCode::ServerFailure,
                                },
                            )
                            .build_into(response);

                            return;
                        }
                    };
            }
            builder.build_into(response);

            if cfg!(debug_assertions) {
                //print_buffer("Output", &response);
            }
        }
        _ => {
            DNSPacketBuilder::respond(&packet, ResponseCode::NotImplemented).build_into(response);
        }
    }
}

#[derive(Debug, Error)]
enum ForwardError {
    #[error(transparent)]
    ParsePacket(#[from] proto::PacketError),
    #[error(transparent)]
    IO(#[from] std::io::Error),
}

async fn forward_request<'a, 'b>(
    socket: &tokio::net::UdpSocket,
    packet: &proto::Packet<'a>,
    questions: &[proto::Question<'a>],
    mut builder: DNSPacketBuilder,
) -> Result<DNSPacketBuilder, ForwardError>
where
    'a: 'b,
{
    let mut request = ArrayBuffer::new().with_max_len((u16::MAX - 2) as usize);
    let mut response = [0; 1024];

    for q in questions {
        let name: DomainName = (&q.name()).into();
        DNSPacketBuilder::query(packet.header().id())
            .add_question(Question::new(
                q.q_type().clone(),
                q.q_class().clone(),
                name.clone(),
            ))
            .build_into(&mut request);

        //print_buffer("Forward Request", &request);

        socket.send(&request).await?;
        let resp_size = socket.recv(&mut response).await?;

        //print_buffer("Forward Response", &ArrayBuffer::from(&response[..resp_size]));

        let Some(res_packet) = proto::Packet::parse(&response[..resp_size], 0)? else {
            continue;
        };

        assert_eq!(packet.header().id(), res_packet.header().id());

        builder = res_packet
            .answers()
            .filter(|answer| name.equals(&answer.name()))
            .fold(
                builder.add_question(Question::new(
                    q.q_type().clone(),
                    q.q_class().clone(),
                    name.clone(),
                )),
                |b, a| b.add_answer(Resource(name.clone(), a.into())),
            );
    }

    Ok(builder)
}
/*
#[cfg(debug_assertions)]
fn print_buffer(label: &str, buffer: &ArrayBuffer) {
    eprintln!("--- Begin {label} ---");
    eprint!("{buffer:b}");
    eprintln!("--- End {label} ---");
}*/
