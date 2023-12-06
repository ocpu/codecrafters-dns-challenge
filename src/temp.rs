struct DNSPacketId(u16);

enum DNSPacket {
    Query(DNSQuery),
    InverseQuery,
    QueryResponse,
    StatusRequest,
    StatusResponse
}

trait IntoResource {
    fn into_resource()
}

struct DNSQuery {
    id: DNSPacketId,
    recursion: bool,
    questions: Vec<Question>
}
