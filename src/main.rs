use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio::{
    sync::mpsc,
};

use clap::Parser;
use thiserror::Error;
use tokio::net::UdpSocket;
use tracing::{Instrument, Level};

use array_buffer::ArrayBuffer;
use packet::DNSPacketBuilder;
use proto::{FromPacketBytes, Opcode};

use crate::cache::EVCache;
use crate::{domain_name::DomainName, proto::ResponseCode, question::Question, resource::Resource};

mod array_buffer;
mod cache;
mod domain_name;
mod header;
mod label;
mod packet;
mod proto;
mod question;
mod resource;
mod types;

#[cfg(feature = "code_crafters")]
const DEFAULT_PORT: u16 = 2053;
#[cfg(not(feature = "code_crafters"))]
const DEFAULT_PORT: u16 = 53;

const DEFAULT_UPSTREAM: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 53);

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The resolver to use
    #[arg(short, long, default_value_t = DEFAULT_UPSTREAM)]
    resolver: SocketAddr,

    /// More output
    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    /// Debug output
    #[arg[long, default_value_t = false]]
    vvv: bool,

    /// The port to listen on
    #[arg(short, long, default_value_t = DEFAULT_PORT)]
    port: u16,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = Args::parse();

    // Setup logging
    let max_level = if args.vvv {
        Level::DEBUG
    } else if args.verbose {
        Level::INFO
    } else {
        Level::WARN
    };

    configure_tracing(max_level);

    // Setup cache
    let (cache, cache_operator) = cache::new();
    tokio::spawn(cache_operator.listen());

    // Code Crafters cache entries
    #[cfg(feature = "code_crafters")]
    setup_for_code_crafters(&cache).await;

    // UDP Listener
    let (mut udp, rx) = match UDPStateSender::new(args.port, args.resolver).await {
        Ok(res) => res,
        Err(_) => {
            tracing::error!(
                transport = "UDP",
                port = args.port,
                "Failed to bind listener"
            );
            return;
        }
    };
    tracing::info!(transport = "UDP", port = args.port, "Listening");
    spawn_udp_handler(cache.clone(), rx);

    // Handle exit signal
    let (sigint_sender, mut sigint_reciever) = tokio::sync::broadcast::channel(1);
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen on Ctrl+c");
        sigint_sender.send(())
    });

    let mut udp_buffer = [0; 1024];
    loop {
        tokio::select! {
            (size, source) = udp.recv(&mut udp_buffer) => {
                udp.enqueue(&udp_buffer[..size], source, |rx| spawn_udp_handler(cache.clone(), rx)).await;
            }
            _ = sigint_reciever.recv() => break,
        }
    }

    tracing::info!("Closing server");
}

// NOTE: An owned EVCache is needed to have its own read handle on the cache data.
fn spawn_udp_handler(cache: EVCache, mut rx: mpsc::Receiver<UDPState>) {
    tokio::spawn(async move {
        let mut response = ArrayBuffer::new().with_max_len(512);
        while let Some(state) = rx.recv().await {
            response.clear();
            async {
                handle_dns_packet(state.buffer, &mut response, &state.forwarding, &cache).await;
                if response.len() > 0 {
                    if let Err(_) = state
                        .socket
                        .send_to(response.as_slice(), state.source)
                        .await
                    {
                        tracing::error!("Failed to send back to source");
                    }
                }
            }
            .instrument(tracing::info_span!("dns_request", source = %state.source))
            .await
        }
    });
}




        }
    }
}

#[cfg(feature = "code_crafters")]
async fn setup_for_code_crafters(cache: &EVCache) {
    use create::resource::ResourceData;

    cache
        .bulk()
        .insert(
            &DomainName::from_static("codecrafters.io"),
            ResourceData::A {
                ttl: 500,
                addr: [8, 8, 8, 8].into(),
            },
        )
        .await
        .unwrap()
        .insert(
            &DomainName::from_static("codecrafters.io"),
            ResourceData::A {
                ttl: 500,
                addr: [8, 8, 4, 4].into(),
            },
        )
        .await
        .unwrap()
        .publish()
        .await
        .unwrap();
}

struct UDPState {
    socket: Arc<UdpSocket>,
    forwarding: Arc<SocketAddr>,
    buffer: ArrayBuffer,
    source: SocketAddr,
}

struct UDPStateSender {
    socket: Arc<UdpSocket>,
    forwarding: Arc<SocketAddr>,
    sender: mpsc::Sender<UDPState>,
    port: u16,
}

impl UDPStateSender {
    pub async fn new(
        port: u16,
        forwarding_addr: SocketAddr,
    ) -> Result<(Self, mpsc::Receiver<UDPState>), std::io::Error> {
        let (tx, rx) = mpsc::channel(1000);
        let udp_socket = Arc::new(UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], port))).await?);

        Ok((
            Self {
                socket: udp_socket,
                forwarding: Arc::new(forwarding_addr),
                sender: tx,
                port,
            },
            rx,
        ))
    }

    pub async fn recv(&mut self, buffer: &mut [u8]) -> (usize, SocketAddr) {
        let mut retried = false;
        loop {
            match self.socket.recv_from(buffer).await {
                Ok(res) => return res,
                Err(_) if !retried => {
                    retried = true;
                    self.socket = Arc::new(
                        UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], self.port)))
                            .await
                            .expect("Failed to bind to address"),
                    );
                }
                Err(e) => panic!("{e:?}"),
            }
        }
    }

    pub async fn enqueue(
        &mut self,
        buf: impl Into<ArrayBuffer>,
        source: SocketAddr,
        respawn_udp_handler: impl FnOnce(mpsc::Receiver<UDPState>) -> (),
    ) {
        let res = self
            .sender
            .send(UDPState {
                socket: Arc::clone(&self.socket),
                forwarding: Arc::clone(&self.forwarding),
                buffer: buf.into(),
                source,
            })
            .await;
        if let Err(mpsc::error::SendError(state)) = res {
            let (tx, rx) = mpsc::channel(1000);
            self.sender = tx;
            respawn_udp_handler(rx);
            self.sender
                .send(state)
                .await
                .expect("UDP message handler is unable to start")
        }
    }
}

fn configure_tracing(max_level: Level) {
    use tracing_subscriber::{filter::LevelFilter, layer::SubscriberExt, util::SubscriberInitExt};

    #[cfg(feature = "tokio_debug")]
    let subscriber = tracing_subscriber::registry().with(console_subscriber::spawn());

    #[cfg(not(feature = "tokio_debug"))]
    let subscriber = tracing_subscriber::registry();

    subscriber
        .with(LevelFilter::from_level(max_level))
        .with(tracing_subscriber::fmt::layer())
        .init();
}

async fn handle_dns_packet(
    buf: ArrayBuffer,
    response: &mut ArrayBuffer,
    forwarding_addr: &SocketAddr,
    cache: &EVCache,
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
                tracing::info!(section = "question", domain_name = %q.name(), r#type = ?q.q_type(), class = ?q.q_class());
                let name = (&q.name()).into();
                match cache.get((&name, q.q_type())) {
                    Some(records) if !records.is_empty() => {
                        builder = records.iter().fold(
                            builder.add_question(Question::new(
                                q.q_type().clone(),
                                q.q_class().clone(),
                                name.clone(),
                            )),
                            |b, record| b.add_answer(Resource(name.clone(), record.clone())),
                        )
                    }
                    Some(_) => {
                        builder = builder.add_question(Question::new(
                            q.q_type().clone(),
                            q.q_class().clone(),
                            name.clone(),
                        ))
                    }
                    None => unknown_questions.push(q),
                }
            }
            if !unknown_questions.is_empty() {
                builder =
                    match forward_request(&forwarding_addr, &packet, &unknown_questions, builder)
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
    resolver: &SocketAddr,
    packet: &proto::Packet<'a>,
    questions: &[proto::Question<'a>],
    mut builder: DNSPacketBuilder,
) -> Result<DNSPacketBuilder, ForwardError>
where
    'a: 'b,
{
    let mut request = ArrayBuffer::new().with_max_len((u16::MAX - 2) as usize);
    let mut response = [0; 1024];
    let socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
    socket.connect(resolver).await?;

    for q in questions {
        let name: DomainName = (&q.name()).into();
        request.clear();
        DNSPacketBuilder::query(packet.header().id())
            .add_question(Question::new(
                q.q_type().clone(),
                q.q_class().clone(),
                name.clone(),
            ))
            .build_into(&mut request);

        tracing::info!(%name, "Forwarding question");

        //print_buffer("Forward Request", &request);

        socket.send(&request).await?;
        let resp_size = socket.recv(&mut response).await?;

        //print_buffer("Forward Response", &ArrayBuffer::from(&response[..resp_size]));

        let Some(res_packet) = proto::Packet::parse(&response[..resp_size], 0)? else {
            tracing::warn!("Returned no packet repr from response");
            continue;
        };

        assert_eq!(packet.header().id(), res_packet.header().id());
        //println!("name={name}");
        //println!("{res_packet:#?}");

        builder = res_packet
            .answers()
            .filter(|answer| name.equals(&answer.name()))
            .fold(
                builder.add_question(Question::new(
                    q.q_type().clone(),
                    q.q_class().clone(),
                    name.clone(),
                )),
                |b, a| b.add_answer(Resource(name.clone(), Arc::new(a.into()))),
            );
    }

    Ok(builder)
}
/*
fn print_buffer(label: &str, buffer: &ArrayBuffer) {
    eprintln!("--- Begin {label} ---");
    eprint!("{buffer:b}");
    eprintln!("--- End {label} ---");
}*/
