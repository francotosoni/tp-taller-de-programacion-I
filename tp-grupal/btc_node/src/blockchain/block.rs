use super::txs::{Tx, Txs};
use bitcoin_hashes::{sha256d, Hash};
//use std::mem;

use crate::block_header::BlockHeader;
use crate::constants::{GENESIS_BLOCK_HASH_VALUE, GENESIS_BLOCK_MERKLE_ROOT_HASH_VALUE};
use crate::protocol_error::ProtocolError;
use crate::utils::decode_hex;
pub const SIZE_BLOCKS: usize = 48;

#[derive(Debug, Clone)]
pub struct Block {
    pub version: i32,
    pub hash: [u8; 32],
    pub merkle_root_hash: [u8; 32],
    pub timestamp: u32,
    pub bits: u32,
    pub nonce: u32,
    pub txs: Option<Txs>,
}

impl Block {
    pub fn default() -> Block {
        let merkle_root_hash = decode_hex(GENESIS_BLOCK_MERKLE_ROOT_HASH_VALUE);
        let hash = decode_hex(GENESIS_BLOCK_HASH_VALUE);

        Block {
            version: 1,
            hash,
            merkle_root_hash,
            timestamp: 1231006505,
            bits: 0x1d00ffff,
            nonce: 0x18aea41a,
            txs: None,
        }
    }

    pub fn from_block_header(new_block: BlockHeader) -> Block {
        Block {
            version: new_block.version,
            hash: new_block.hash(),
            merkle_root_hash: new_block.merkle_root_hash,
            timestamp: new_block.timestamp,
            bits: new_block.bits,
            nonce: new_block.nonce,
            txs: None,
        }
    }

    pub fn to_block_header(block: Block, prev_block_hash: [u8; 32]) -> BlockHeader {
        BlockHeader {
            version: block.version,
            prev_block_hash,
            merkle_root_hash: block.merkle_root_hash,
            timestamp: block.timestamp,
            bits: block.bits,
            nonce: block.nonce,
        }
    }

    pub fn add_txs(&mut self, txs: Txs) {
        self.txs = Some(txs);
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes.extend_from_slice(&self.merkle_root_hash);
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        bytes.extend_from_slice(&self.bits.to_le_bytes());
        bytes.extend_from_slice(&self.nonce.to_le_bytes());

        bytes
    }

    pub fn from_bytes(
        bytes: [u8; SIZE_BLOCKS],
        prev_hash: [u8; 32],
    ) -> Result<Block, ProtocolError> {
        let version = i32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let merkle_root_hash = bytes[4..36].try_into().unwrap();
        let timestamp = u32::from_le_bytes(bytes[36..40].try_into().unwrap());
        let bits = u32::from_le_bytes(bytes[40..44].try_into().unwrap());
        let nonce = u32::from_le_bytes(bytes[44..48].try_into().unwrap());

        let b = [&bytes[0..4], &prev_hash[..], &bytes[4..48]].concat();
        let hash = sha256d::Hash::hash(&b).to_byte_array();

        Ok(Block {
            version,
            hash,
            merkle_root_hash,
            timestamp,
            bits,
            nonce,
            txs: None,
        })
    }

    pub fn get_tx(&self, txid: [u8; 32]) -> Option<Tx> {
        if let Some(i) = &self.txs {
            return i.get_tx(txid);
        }
        None
    }

    pub fn get_tx_history(&self, pkhash: &Vec<u8>) -> Vec<Tx> {
        if let Some(tx) = &self.txs {
            return tx.get_txs_by_pkhash(pkhash);
        }
        vec![]
    }
}
