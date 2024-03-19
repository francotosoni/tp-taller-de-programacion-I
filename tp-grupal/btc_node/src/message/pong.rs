use crate::{
    message::ping::PingMessage, message_header::MessageHeader, protocol_error::ProtocolError,
};

use std::io::{Read, Write};

#[derive(Debug)]
pub struct PongMessage {
    nonce: u64,
}

impl PongMessage {
    pub fn new(nonce: u64) -> PongMessage {
        PongMessage { nonce }
    }

    pub fn read_from(stream: &mut dyn Read) -> Result<PongMessage, ProtocolError> {
        let mut nonce = [0u8; 8];
        stream.read_exact(&mut nonce)?;
        Ok(PongMessage {
            nonce: u64::from_le_bytes(nonce),
        })
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.nonce.to_le_bytes());

        bytes
    }

    pub fn write_to(&self, stream: &mut dyn Write) -> Result<(), ProtocolError> {
        let payload = self.to_bytes();

        let header = MessageHeader::new("pong".to_string(), payload.clone())?;
        header.write_to(stream)?;

        stream.write_all(&payload[..])?;
        Ok(())
    }

    pub fn compare_with_ping(&self, ping: PingMessage) -> bool {
        ping.get_nonce() == (self.nonce)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    #[test]
    fn test_pong_message_read_from() {
        let nonce: u64 = 123456;
        let buffer: Vec<u8> = nonce.to_le_bytes().to_vec();

        let result = PongMessage::read_from(&mut Cursor::new(&buffer));
        assert!(result.is_ok(), "Error at reading: {:?}", result);

        let parsed_message = result.unwrap();
        assert_eq!(parsed_message.nonce, nonce);
    }

    #[test]
    fn test_pong_message_compare_with_ping() {
        let pong_nonce: u64 = 123456;
        let ping_nonce: u64 = 123456;
        let pong_message = PongMessage::new(pong_nonce);
        let ping_message = PingMessage::new(ping_nonce);

        assert!(pong_message.compare_with_ping(ping_message));
    }

    #[test]
    fn test_pong_message_to_bytes() {
        let nonce: u64 = 123456;
        let message = PongMessage::new(nonce);

        let expected_bytes = vec![64, 226, 1, 0, 0, 0, 0, 0];
        let actual_bytes = message.to_bytes();
        assert_eq!(actual_bytes, expected_bytes);
    }
}
