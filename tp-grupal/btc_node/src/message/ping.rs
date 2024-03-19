use crate::{message_header::MessageHeader, protocol_error::ProtocolError};

use std::io::{Read, Write};

use super::Serializable;

#[derive(Debug)]
pub struct PingMessage {
    pub nonce: u64,
}

impl Serializable for PingMessage {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.nonce.to_le_bytes());

        bytes
    }
}

impl PingMessage {
    pub fn new(nonce: u64) -> PingMessage {
        PingMessage { nonce }
    }

    pub fn read_from(stream: &mut dyn Read) -> Result<PingMessage, ProtocolError> {
        let mut nonce = [0u8; 8];
        stream.read_exact(&mut nonce)?;

        Ok(PingMessage {
            nonce: u64::from_le_bytes(nonce),
        })
    }

    pub fn write_to(&self, stream: &mut dyn Write) -> Result<(), ProtocolError> {
        let payload = self.to_bytes();

        let header = MessageHeader::new("ping".to_string(), payload.clone())?;
        header.write_to(stream)?;

        stream.write_all(&payload[..])?;
        Ok(())
    }

    pub fn get_nonce(&self) -> u64 {
        self.nonce
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    #[test]
    fn test_ping_message_read_from() {
        let nonce: u64 = 123456;
        let buffer: Vec<u8> = nonce.to_le_bytes().to_vec();

        let result = PingMessage::read_from(&mut Cursor::new(&buffer));
        assert!(result.is_ok(), "Error at reading {:?}", result);

        let parsed_message = result.unwrap();
        assert_eq!(parsed_message.nonce, nonce);
    }

    #[test]
    fn test_ping_message_to_bytes() {
        let nonce: u64 = 123456;
        let message = PingMessage::new(nonce);

        let expected_bytes = vec![64, 226, 1, 0, 0, 0, 0, 0];
        let actual_bytes = message.to_bytes();
        assert_eq!(actual_bytes, expected_bytes);
    }
}
