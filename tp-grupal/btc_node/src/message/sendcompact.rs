use super::Serializable;
use crate::protocol_error::ProtocolError;
use std::io::Read;

#[derive(Debug)]
pub struct SendCompactMessage {
    announce: [u8; 1],
    version: [u8; 8],
}

impl SendCompactMessage {
    pub fn read_from(stream: &mut dyn Read) -> Result<SendCompactMessage, ProtocolError> {
        let mut announce = [0u8; 1];
        stream.read_exact(&mut announce)?;

        let mut version = [0u8; 8];
        stream.read_exact(&mut version)?;

        Ok(SendCompactMessage { announce, version })
    }
}

impl Serializable for SendCompactMessage {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.announce);
        bytes.extend_from_slice(&self.version);

        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_send_compact_message_read_from() {
        let data = [1u8, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut cursor = Cursor::new(&data[..]);

        let result = SendCompactMessage::read_from(&mut cursor);

        assert!(result.is_ok(), "Error al leer desde el flujo: {:?}", result);

        let send_compact_message = result.unwrap();

        assert_eq!(send_compact_message.announce, [1]);
        assert_eq!(send_compact_message.version, [2, 3, 4, 5, 6, 7, 8, 9]);
    }
}
