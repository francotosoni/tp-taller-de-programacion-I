use core::fmt;
use std::io::{Read, Write};

use crate::{
    message::compact_size::CompactSize, message::inventory::Inventory,
    message_header::MessageHeader, protocol_error::ProtocolError, utils::bytes_to_hex_string,
};

use super::Serializable;

#[derive(Debug)]
pub struct InvMessage {
    pub count: CompactSize,
    pub inventory: Vec<Inventory>,
}

impl fmt::Display for InvMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[ ")?;
        for inv in &self.inventory {
            write!(
                f,
                "({}: {}) ",
                inv.type_identifier,
                bytes_to_hex_string(&inv.hash[0..3])
            )?;
        }
        write!(f, "]")
    }
}

impl InvMessage {
    pub fn read_from(stream: &mut dyn Read) -> Result<InvMessage, ProtocolError> {
        let count = CompactSize::read_from(stream)?;

        let mut inventory: Vec<Inventory> = Vec::new();

        for _ in 0..count.into_inner() {
            inventory.push(Inventory::read_from(stream)?);
        }

        Ok(InvMessage { count, inventory })
    }

    pub fn write_to(&self, stream: &mut dyn Write) -> Result<(), ProtocolError> {
        let bytes = self.to_bytes();

        let message_header = MessageHeader::new("inv".to_string(), bytes.clone())?;
        message_header.write_to(stream)?;

        stream.write_all(&bytes)?;

        Ok(())
    }
}

impl Serializable for InvMessage {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.count.to_le_bytes());
        for i in &self.inventory {
            bytes.extend_from_slice(&i.to_bytes());
        }

        bytes
    }
}
