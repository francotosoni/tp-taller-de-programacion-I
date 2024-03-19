use crate::{
    message_header::MessageHeader, protocol_error::ProtocolError, raw_transaction::RawTransaction,
};

use std::io::{Read, Write};

use super::Serializable;

#[derive(Debug)]
pub struct TxMessage {
    pub tx: RawTransaction,
}

impl TxMessage {
    pub fn new(tx: RawTransaction) -> TxMessage {
        TxMessage { tx }
    }

    pub fn read_from(stream: &mut dyn Read) -> Result<TxMessage, ProtocolError> {
        let tx = RawTransaction::read_from(stream)?;

        Ok(TxMessage { tx })
    }

    pub fn write_to(&self, stream: &mut dyn Write) -> Result<(), ProtocolError> {
        let payload = self.to_bytes();

        let header = MessageHeader::new("tx".to_string(), payload.clone())?;
        header.write_to(stream)?;

        stream.write_all(&payload[..])?;
        Ok(())
    }
}

impl Serializable for TxMessage {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.tx.to_bytes());

        bytes
    }
}
