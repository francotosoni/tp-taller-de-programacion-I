use std::io::Read;
use std::io::Write;

use crate::message_header::MessageHeader;
use crate::{
    message::compact_size::CompactSize, message::inventory::Inventory,
    message::inventory::TypeIdentifier, protocol_error::ProtocolError,
};

use super::Serializable;

#[derive(Debug)]
pub struct GetDataMessage {
    pub count: CompactSize,
    pub inventory: Vec<Inventory>,
}

impl Serializable for GetDataMessage {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.count.to_le_bytes());
        for i in &self.inventory {
            bytes.extend_from_slice(&i.to_bytes());
        }

        bytes
    }
}

impl GetDataMessage {
    pub fn new(hash: Vec<[u8; 32]>, t: TypeIdentifier) -> GetDataMessage {
        let count = CompactSize::new_from_usize(hash.len());
        let mut inventory = Vec::new();
        for h in &hash {
            inventory.push(Inventory::new(t.clone(), *h));
        }

        GetDataMessage { count, inventory }
    }

    pub fn new_from_inventory(inventory: Vec<Inventory>) -> GetDataMessage {
        let count = CompactSize::new_from_usize(inventory.len());
        GetDataMessage { count, inventory }
    }

    pub fn read_from(stream: &mut dyn Read) -> Result<GetDataMessage, ProtocolError> {
        let count = CompactSize::read_from(stream)?;

        let mut inventory: Vec<Inventory> = Vec::new();

        for _ in 0..count.into_inner() {
            inventory.push(Inventory::read_from(stream)?);
        }

        Ok(GetDataMessage { count, inventory })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.count.to_le_bytes());
        for i in &self.inventory {
            bytes.extend_from_slice(&i.to_bytes());
        }

        bytes
    }

    pub fn write_to(&self, stream: &mut dyn Write) -> Result<(), ProtocolError> {
        let bytes = self.to_bytes();

        let message_header = MessageHeader::new("getdata".to_string(), bytes.clone())?;
        message_header.write_to(stream)?;

        stream.write_all(&bytes)?;

        Ok(())
    }
}
