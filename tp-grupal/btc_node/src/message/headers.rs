use std::io::{Read, Write};

use crate::{
    block_header::BlockHeader, message::compact_size::CompactSize, message_header::MessageHeader,
    protocol_error::ProtocolError,
};

use super::Serializable;

pub mod headers_message_builder {
    use super::*;
    pub struct HeadersMessageBuilder {
        count: Option<CompactSize>,
        headers: Option<Vec<BlockHeader>>,
    }

    impl Default for HeadersMessageBuilder {
        fn default() -> Self {
            Self::new()
        }
    }

    impl HeadersMessageBuilder {
        pub fn new() -> Self {
            Self {
                count: None,
                headers: None,
            }
        }

        pub fn count(mut self, count: CompactSize) -> Self {
            self.count = Some(count);
            self
        }

        pub fn headers(mut self, headers: Vec<BlockHeader>) -> Self {
            self.headers = Some(headers);
            self
        }

        pub fn build(self) -> Result<HeadersMessage, String> {
            Ok(HeadersMessage {
                count: self.count.ok_or("count not set")?,
                headers: self.headers.ok_or("headers not set")?,
            })
        }
    }
}

#[derive(Debug)]
pub struct HeadersMessage {
    pub count: CompactSize,
    pub headers: Vec<BlockHeader>,
}

impl HeadersMessage {
    pub fn new(headers: Vec<BlockHeader>) -> HeadersMessage {
        HeadersMessage {
            count: CompactSize::new_from_usize(headers.len()),
            headers,
        }
    }

    pub fn read_from(stream: &mut dyn Read) -> Result<HeadersMessage, ProtocolError> {
        let header_count = CompactSize::read_from(stream)?;

        let mut headers: Vec<BlockHeader> = Vec::new();
        let mut transaction_count = [0u8; 1];

        for _ in 0..header_count.into_inner() {
            headers.push(BlockHeader::read_from(stream)?);

            stream.read_exact(&mut transaction_count)?;
        }

        let header_message = headers_message_builder::HeadersMessageBuilder::new()
            .count(header_count)
            .headers(headers)
            .build()?;

        Ok(header_message)
    }

    pub fn write_to(&self, stream: &mut dyn Write) -> Result<(), ProtocolError> {
        let payload = self.to_bytes();

        let header = MessageHeader::new("headers".to_string(), payload.clone())?;
        header.write_to(stream)?;

        stream.write_all(&payload[..])?;
        Ok(())
    }
}

impl Serializable for HeadersMessage {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.count.to_le_bytes());
        for header in &self.headers {
            bytes.extend_from_slice(&header.to_bytes());
            bytes.extend_from_slice(&[0u8]);
        }

        bytes
    }
}
