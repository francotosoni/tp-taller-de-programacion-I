use crate::protocol_error::ProtocolError;
use bitcoin_hashes::{sha256d, Hash};
use std::io::Read;

pub mod block_header_builder {
    use super::*;
    pub struct BlockHeaderBuilder {
        version: Option<i32>,
        prev_block_hash: Option<[u8; 32]>,
        merkle_root_hash: Option<[u8; 32]>,
        timestamp: Option<u32>,
        bits: Option<u32>,
        nonce: Option<u32>,
    }

    impl Default for BlockHeaderBuilder {
        fn default() -> Self {
            Self::new()
        }
    }

    impl BlockHeaderBuilder {
        pub fn new() -> Self {
            Self {
                version: None,
                prev_block_hash: None,
                merkle_root_hash: None,
                timestamp: None,
                bits: None,
                nonce: None,
            }
        }

        pub fn version(mut self, version: i32) -> Self {
            self.version = Some(version);
            self
        }

        pub fn prev_block_hash(mut self, prev_block_hash: [u8; 32]) -> Self {
            self.prev_block_hash = Some(prev_block_hash);
            self
        }

        pub fn merkle_root_hash(mut self, merkle_root_hash: [u8; 32]) -> Self {
            self.merkle_root_hash = Some(merkle_root_hash);
            self
        }

        pub fn timestamp(mut self, timestamp: u32) -> Self {
            self.timestamp = Some(timestamp);
            self
        }

        pub fn bits(mut self, bits: u32) -> Self {
            self.bits = Some(bits);
            self
        }

        pub fn nonce(mut self, nonce: u32) -> Self {
            self.nonce = Some(nonce);
            self
        }

        pub fn build(self) -> Result<BlockHeader, String> {
            Ok(BlockHeader {
                version: self.version.ok_or("version not set")?,
                prev_block_hash: self.prev_block_hash.ok_or("prev_block_hash not set")?,
                merkle_root_hash: self.merkle_root_hash.ok_or("merkle_root_hash not set")?,
                timestamp: self.timestamp.ok_or("timestamp not set")?,
                bits: self.bits.ok_or("bits not set")?,
                nonce: self.nonce.ok_or("nonce not set")?,
            })
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlockHeader {
    pub version: i32,
    pub prev_block_hash: [u8; 32],
    pub merkle_root_hash: [u8; 32],
    pub timestamp: u32,
    pub bits: u32,
    pub nonce: u32,
}

impl BlockHeader {
    pub fn read_from(stream: &mut dyn Read) -> Result<BlockHeader, ProtocolError> {
        let mut version = [0u8; 4];
        stream.read_exact(&mut version)?;

        let mut prev_block_hash = [0u8; 32];
        stream.read_exact(&mut prev_block_hash)?;

        let mut merkle_root_hash = [0u8; 32];
        stream.read_exact(&mut merkle_root_hash)?;

        let mut timestamp = [0u8; 4];
        stream.read_exact(&mut timestamp)?;

        let mut bits = [0u8; 4];
        stream.read_exact(&mut bits)?;

        let mut nonce = [0u8; 4];
        stream.read_exact(&mut nonce)?;

        let block_header = block_header_builder::BlockHeaderBuilder::new()
            .version(i32::from_le_bytes(version))
            .prev_block_hash(prev_block_hash)
            .merkle_root_hash(merkle_root_hash)
            .timestamp(u32::from_le_bytes(timestamp))
            .bits(u32::from_le_bytes(bits))
            .nonce(u32::from_le_bytes(nonce))
            .build()?;

        if !block_header.validate_proof_of_work() {
            return Err(ProtocolError::Error(
                "this block header failed the proof of work".to_string(),
            ));
        }

        Ok(block_header)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes.extend_from_slice(&self.prev_block_hash);
        bytes.extend_from_slice(&self.merkle_root_hash);
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        bytes.extend_from_slice(&self.bits.to_le_bytes());
        bytes.extend_from_slice(&self.nonce.to_le_bytes());

        bytes
    }

    pub fn hash(&self) -> [u8; 32] {
        sha256d::Hash::hash(&self.to_bytes()[..]).to_byte_array()
    }

    pub fn validate_proof_of_work(&self) -> bool {
        let nbits: [u8; 4] = self.bits.to_be_bytes();

        let exponent: [u8; 4] = [nbits[0], 0, 0, 0];
        let mantissa: [u8; 4] = [0, nbits[1], nbits[2], nbits[3]];

        let zeros_to_right = (8 * (u32::from_le_bytes(exponent) - 3)) / 8;
        let zeros_to_left: u32 = 32 - 3 - zeros_to_right;

        let mut target_threshold: Vec<u8> = Vec::with_capacity(32);

        for _ in 0..zeros_to_left {
            target_threshold.push(0u8);
        }

        for item in mantissa.iter().skip(1) {
            target_threshold.push(*item);
        }

        for _ in 0..zeros_to_right {
            target_threshold.push(0u8);
        }

        let hash: [u8; 32] = self.hash();

        for i in 0..32 {
            match (hash[31 - i], target_threshold[i]) {
                (hash_val, target_val) if hash_val < target_val => return true,
                (hash_val, target_val) if hash_val > target_val => return false,
                _ => continue,
            }
        }

        false
    }
}
