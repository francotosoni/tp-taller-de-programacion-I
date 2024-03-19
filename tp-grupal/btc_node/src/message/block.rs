use crate::{
    block_header::BlockHeader, message::compact_size::CompactSize, message_header::MessageHeader,
    protocol_error::ProtocolError, raw_transaction::RawTransaction,
};

use std::io::{Read, Write};

use super::Serializable;

#[derive(Debug, Clone)]
pub struct BlockMessage {
    pub block_header: BlockHeader,
    pub txn_count: CompactSize,
    pub txns: Vec<RawTransaction>,
}

impl Serializable for BlockMessage {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.block_header.to_bytes());
        bytes.extend_from_slice(&self.txn_count.to_le_bytes());
        for txn in &self.txns {
            bytes.extend_from_slice(&txn.to_bytes());
        }
        bytes
    }
}

impl BlockMessage {
    pub fn read_from(stream: &mut dyn Read) -> Result<BlockMessage, ProtocolError> {
        let block_header = BlockHeader::read_from(stream)?;
        let txn_count = CompactSize::read_from(stream)?;
        let mut txns: Vec<RawTransaction> = Vec::new();

        for _ in 0..txn_count.into_inner() {
            txns.push(RawTransaction::read_from(stream)?);
        }

        Ok(BlockMessage {
            block_header,
            txn_count,
            txns,
        })
    }

    pub fn get_txns_hashes(&self) -> Vec<[u8; 32]> {
        let mut txns_hashes = Vec::new();
        for txn in &self.txns {
            txns_hashes.push(txn.get_tx_id());
        }
        txns_hashes
    }

    pub fn write_to(&self, stream: &mut dyn Write) -> Result<(), ProtocolError> {
        let payload = self.to_bytes();

        let header = MessageHeader::new(String::from("block"), payload.clone())?;
        header.write_to(stream)?;

        stream.write_all(&payload[..])?;
        Ok(())
    }
}
