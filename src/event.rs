use hyper::body::Chunk;
pub struct Event {
    pub event: String,
    pub data: Vec<u8>,
    pub id: String,
    pub retry: u8
}