use crate::{message_header::MessageHeader, protocol_error::ProtocolError};
use std::io::{Read, Write};

use super::Serializable;

#[derive(Debug)]
pub struct FeeFilterMessage {
    feerate: u64,
}

impl FeeFilterMessage {
    pub fn read_from(stream: &mut dyn Read) -> Result<FeeFilterMessage, ProtocolError> {
        let mut feerate = [0u8; 8];
        stream.read_exact(&mut feerate)?;

        Ok(FeeFilterMessage {
            feerate: u64::from_le_bytes(feerate),
        })
    }

    pub fn write_to(&self, stream: &mut dyn Write) -> Result<(), ProtocolError> {
        let payload = self.to_bytes();

        let header = MessageHeader::new("feefilter".to_string(), payload.clone())?;
        header.write_to(stream)?;

        stream.write_all(&payload[..])?;
        Ok(())
    }
}

impl Serializable for FeeFilterMessage {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.feerate.to_le_bytes());

        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    #[test]
    fn test_fee_filter_message_read_from() {
        let data = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let mut cursor = Cursor::new(&data[..]);

        let result = FeeFilterMessage::read_from(&mut cursor);

        assert!(result.is_ok(), "Error at reading: {:?}", result);

        let fee_filter_message = result.unwrap();

        assert_eq!(fee_filter_message.feerate, 578437695752307201);
    }
}
