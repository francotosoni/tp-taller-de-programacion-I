use crate::{
    message::compact_size::CompactSize, message_header::MessageHeader,
    protocol_error::ProtocolError,
};

use std::io::{Read, Write};

use super::Serializable;

#[derive(Debug)]
pub struct GetHeadersMessage {
    version: u32,
    pub hash_count: CompactSize,
    pub block_header_hashes: Vec<[u8; 32]>,
    stop_hash: [u8; 32],
}

impl Serializable for GetHeadersMessage {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes.extend_from_slice(&self.hash_count.to_le_bytes());
        bytes.extend_from_slice(&self.block_header_hashes[0]);
        bytes.extend_from_slice(&self.stop_hash);

        bytes
    }
}

impl GetHeadersMessage {
    pub fn new(hash_bytes: [u8; 32]) -> GetHeadersMessage {
        GetHeadersMessage {
            version: 70015,
            hash_count: CompactSize::U8(1),
            block_header_hashes: vec![hash_bytes],
            stop_hash: [0u8; 32],
        }
    }

    pub fn write_to(&self, stream: &mut dyn Write) -> Result<(), ProtocolError> {
        let payload = self.to_bytes();

        let header = MessageHeader::new(String::from("getheaders"), payload.clone())?;
        header.write_to(stream)?;

        stream.write_all(&payload[..])?;
        Ok(())
    }

    pub fn read_from(stream: &mut dyn Read) -> Result<GetHeadersMessage, ProtocolError> {
        let mut version = [0u8; 4];
        stream.read_exact(&mut version)?;

        let mut hashes: Vec<[u8; 32]> = Vec::new();
        let count = CompactSize::read_from(stream)?;
        for _ in 0..count.into_inner() {
            let mut hash = [0u8; 32];
            stream.read_exact(&mut hash)?;

            hashes.push(hash);
        }

        let mut stop_hash = [0u8; 32];
        stream.read_exact(&mut stop_hash)?;

        let last_hash: [u8; 32] = match hashes.first() {
            Some(elem) => *elem,
            None => [0u8; 32],
        };

        let get_headers = GetHeadersMessage::new(last_hash);

        Ok(get_headers)
    }
}
