mod block;
pub mod txs;
pub mod utxo_set;

use block::{Block, SIZE_BLOCKS};
use txs::Txs;
use utxo_set::UtxoSet;

use std::collections::LinkedList;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader};

use crate::message::compact_size::CompactSize;
use crate::raw_transaction::RawTransaction;
use crate::utils::decode_hex;
use crate::{
    block_header::BlockHeader, constants::GENESIS_BLOCK_HASH_VALUE, merkle_tree::merkle_tree_root,
    message::block::BlockMessage, protocol_error::ProtocolError,
};

use self::txs::Tx;
use self::utxo_set::Output;

#[derive(Debug, Default)]
pub struct Blockchain {
    chain: LinkedList<Block>,
    pub utxo: UtxoSet,
}

impl Blockchain {
    pub fn new() -> Blockchain {
        let mut chain = LinkedList::new();
        chain.push_front(Block::default());
        Blockchain {
            chain,
            utxo: UtxoSet::default(),
        }
    }

    fn push_block(&mut self, block: Block, prev_hash: [u8; 32]) -> Result<(), ProtocolError> {
        let head = self.chain.front().unwrap();
        if head.hash == prev_hash {
            self.chain.push_front(block);
            return Ok(());
        }

        for b in self.chain.iter().take(100) {
            if b.hash == block.hash {
                println!("YA TENGO ESE BLOQUE");
                return Ok(());
            } else if b.hash == prev_hash {
                println!("FORK");
                return Ok(());
            }
        }
        Ok(())
    }

    pub fn push(&mut self, new_header: BlockHeader) -> Result<(), ProtocolError> {
        let prev_hash = new_header.prev_block_hash;
        let block = Block::from_block_header(new_header);

        self.push_block(block, prev_hash)
    }

    pub fn push_full_block(&mut self, new_block: BlockMessage) -> Result<Block, ProtocolError> {
        let prev_hash = new_block.block_header.prev_block_hash;
        let mut block = Block::from_block_header(new_block.block_header);
        let txs = Txs::from_raw_txs(new_block.txns);

        self.utxo.append(&txs);
        block.add_txs(txs);

        self.push_block(block.clone(), prev_hash)?;
        Ok(block)
    }

    pub fn add_block_txs(&mut self, block_message: BlockMessage) -> Result<(), ProtocolError> {
        let hash = block_message.block_header.hash();
        let txs = Txs::from_raw_txs(block_message.txns);

        for block in self.chain.iter_mut() {
            if block.hash == hash {
                let merkle_root = merkle_tree_root(txs.get_tx_ids());
                if merkle_root == block.merkle_root_hash {
                    self.utxo.append(&txs);
                    block.add_txs(txs);
                    return Ok(());
                }
                return Err(ProtocolError::Error(
                    "Merkle root doesn't match".to_string(),
                ));
            }
        }

        Ok(())
    }

    pub fn get_last_header_hash(&self) -> [u8; 32] {
        self.chain.front().unwrap().hash
    }

    pub fn get_size(&self) -> usize {
        self.chain.len()
    }

    pub fn get_tx(&self, txid: [u8; 32]) -> Option<Tx> {
        for block in self.chain.iter() {
            let tx = block.get_tx(txid);
            if tx.is_some() {
                return tx;
            };
        }
        None
    }

    pub fn read_from_file(filepath: String) -> Result<Blockchain, ProtocolError> {
        let file = File::open(filepath)?;
        let mut reader = BufReader::with_capacity(SIZE_BLOCKS, file);

        let mut blockchain = Blockchain::new();

        let mut last_hash = decode_hex(GENESIS_BLOCK_HASH_VALUE);
        let mut tmp;
        loop {
            let buf = {
                reader.fill_buf()?;
                reader.buffer()
            };
            if buf.len() != SIZE_BLOCKS {
                break;
            }

            let block = Block::from_bytes(buf.clone().try_into().unwrap(), last_hash)?;
            tmp = block.hash;
            blockchain.push_block(block, last_hash).unwrap();
            last_hash = tmp;
            reader.consume(buf.len());
        }

        Ok(blockchain)
    }

    pub fn save_to_file(&self, filepath: String) -> Result<(), ProtocolError> {
        let mut file = OpenOptions::new().create(true).write(true).open(filepath)?;

        for block in self.chain.iter().rev().skip(1) {
            std::io::Write::write(&mut file, &block.to_bytes())?;
        }

        Ok(())
    }

    pub fn get_headers(&self, hash: [u8; 32]) -> Vec<BlockHeader> {
        let mut headers = vec![];
        let mut blocks_left = 2000;
        let mut b = false;
        let mut last_hash = hash;

        for block in self.chain.iter().rev() {
            if block.hash == last_hash {
                b = true;
                continue;
            }

            if b {
                headers.push(Block::to_block_header(block.clone(), last_hash));
                last_hash = block.hash;
                blocks_left -= 1;
                if blocks_left == 0 {
                    return headers;
                }
            }
        }
        headers
    }

    pub fn get_blocks(&self, mut hashes: Vec<[u8; 32]>) -> Vec<BlockMessage> {
        if hashes.is_empty() {
            return vec![];
        }
        hashes.reverse();
        let mut blocks = vec![];

        while !hashes.is_empty() {
            let mut hash = hashes.pop().unwrap();
            let mut last_hash = [0u8; 32];

            for block in self.chain.iter().rev() {
                if block.hash == hash {
                    if let Some(txs) = &block.txs {
                        let txns = txs.to_raw_txs();
                        blocks.push(BlockMessage {
                            block_header: Block::to_block_header(block.clone(), last_hash),
                            txn_count: CompactSize::new_from_usize(txns.len()),
                            txns,
                        });
                    }
                    if hashes.is_empty() {
                        return blocks;
                    }
                    hash = hashes.pop().unwrap();
                }
                last_hash = block.hash;
            }
        }

        blocks
    }

    pub fn get_hashes_since(&self, date: u32) -> Vec<[u8; 32]> {
        let mut hashes = vec![];
        for block in self.chain.iter() {
            if block.timestamp >= date {
                hashes.push(block.hash);
            }
        }
        hashes.reverse();
        hashes
    }

    /// It returns every unspent output in the blockchain that is related to a public key hash.
    pub fn get_utxo(&self, pkhash: Vec<u8>) -> Vec<([u8; 32], Output)> {
        self.utxo.by_pkhash(pkhash)
    }

    /// It returns every transaction in the blockchain that is related to a public key hash.
    pub fn get_tx_history(&self, pkhash: Vec<u8>) -> Vec<Tx> {
        let mut history: Vec<Tx> = vec![];
        for block in self.chain.iter() {
            history.extend_from_slice(&block.get_tx_history(&pkhash));
        }
        history
    }

    /// Checks if a RawTransaction is valid or not.
    /// The inputs of the transaction are valid if they spend outputs in the utxo set.
    /// The amount spendable must not be greater than the amount spent.
    pub fn is_valid_tx(&self, tx: &RawTransaction) -> bool {
        let mut spendable: i64 = 0;
        if tx.tx_in_count.into_inner() == 0 {
            return false;
        }

        for (i, txin) in tx.tx_in.iter().enumerate() {
            let a = self
                .utxo
                .get(txin.previous_output.hash, txin.previous_output.index);
            if let Some(output) = a {
                if !output.pkscript.evaluate(tx.clone(), i) {
                    return false;
                }
                spendable += output.value;
            } else {
                return false;
            };
        }

        let mut spent: i64 = 0;
        for txout in tx.tx_out.iter() {
            spent += txout.value;
        }

        spent <= spendable
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        message::compact_size::CompactSize,
        raw_transaction::{Outpoint, TxIn},
        raw_transaction::{RawTransaction, TxOut},
    };

    use super::*;

    #[test]
    fn test_empty_blockchain() {
        let blockchain = Blockchain::new();
        assert_eq!(blockchain.get_size(), 1);
        // assert_eq!(blockchain.heads.len(), 1);
        assert_eq!(
            blockchain.get_last_header_hash(),
            decode_hex(crate::constants::GENESIS_BLOCK_HASH_VALUE)
        );
    }

    #[test]
    fn test_pushing_to_blockchain() {
        let mut blockchain = Blockchain::new();
        let first_hash = blockchain.get_last_header_hash();

        let block1 = BlockHeader {
            version: 1,
            prev_block_hash: first_hash,
            merkle_root_hash: [0; 32],
            timestamp: 1234567890,
            bits: 0x1d00ffff,
            nonce: 0xabcdef,
        };
        let hash_block1 = block1.hash();
        assert!(blockchain.push(block1).is_ok());
        assert_eq!(blockchain.get_size(), 2);
        assert_eq!(blockchain.get_last_header_hash(), hash_block1);

        let block2 = BlockHeader {
            version: 1,
            prev_block_hash: hash_block1,
            merkle_root_hash: [1; 32],
            timestamp: 1234567012,
            bits: 0x1d00f0ff,
            nonce: 0xacceef,
        };
        let hash_block2 = block2.hash();
        assert!(blockchain.push(block2).is_ok());
        assert_eq!(blockchain.get_size(), 3);
        assert_eq!(blockchain.get_last_header_hash(), hash_block2);

        // assert_eq!(blockchain.heads.len(), 1);

        let block3 = BlockHeader {
            version: 2,
            prev_block_hash: hash_block2,
            merkle_root_hash: [0; 32],
            timestamp: 1234567890,
            bits: 0x1d00ffff,
            nonce: 0x987654,
        };
        let hash_block3 = block3.hash();
        assert!(blockchain.push(block3).is_ok());
        assert_eq!(blockchain.get_size(), 4);
        assert_eq!(blockchain.get_last_header_hash(), hash_block3);

        // assert_eq!(blockchain.heads.len(), 1);
    }

    // #[test]
    // fn test_forks_in_blockchain() {
    //     let mut blockchain = Blockchain::new();
    //
    //     let first_hash = blockchain.get_last_header_hash();
    //
    //     let block1 = BlockHeader {
    //         version: 1,
    //         prev_block_hash: first_hash,
    //         merkle_root_hash: [0; 32],
    //         timestamp: 1234567890,
    //         bits: 0x1d00ffff,
    //         nonce: 0xabcdef,
    //     };
    //     let hash_block1 = block1.hash();
    //
    //     assert!(blockchain.push(block1).is_ok());
    //     assert_eq!(blockchain.get_size(), 2);
    //     assert_eq!(blockchain.get_last_header_hash(), hash_block1);
    //
    //     let block2 = BlockHeader {
    //         version: 1,
    //         prev_block_hash: first_hash,
    //         merkle_root_hash: [1; 32],
    //         timestamp: 1234567012,
    //         bits: 0x1d00f0ff,
    //         nonce: 0xacceef,
    //     };
    //     let hash_block2 = block2.hash();
    //     assert!(blockchain.push(block2).is_ok());
    //     assert_eq!(blockchain.get_size(), 2);
    //     assert_eq!(blockchain.get_last_header_hash(), hash_block1);
    //
    //     // assert_eq!(blockchain.heads.len(), 2);
    //
    //     let block3 = BlockHeader {
    //         version: 2,
    //         prev_block_hash: hash_block2,
    //         merkle_root_hash: [0; 32],
    //         timestamp: 1234567890,
    //         bits: 0x1d00ffff,
    //         nonce: 0x987654,
    //     };
    //     let hash_block3 = block3.hash();
    //     assert!(blockchain.push(block3).is_ok());
    //     assert_eq!(blockchain.get_size(), 3);
    //     assert_eq!(blockchain.get_last_header_hash(), hash_block3);
    //
    //     // let largest_head = blockchain.get_largest_head();
    //     // assert_eq!(largest_head.size, 3);
    //     // assert_eq!(largest_head.block.nonce, 0x987654);
    //     // assert_eq!(blockchain.heads.len(), 2);
    // }

    // #[test]
    // fn test_multiple_forks_in_blockchain() {
    //     let mut blockchain = Blockchain::new();
    //
    //     let first_hash = blockchain.get_last_header_hash();
    //
    //     let block1 = BlockHeader {
    //         version: 1,
    //         prev_block_hash: first_hash,
    //         merkle_root_hash: [0; 32],
    //         timestamp: 1234567890,
    //         bits: 0x1d00ffff,
    //         nonce: 0xabcdef,
    //     };
    //     let hash_block1 = block1.hash();
    //
    //     assert!(blockchain.push(block1).is_ok());
    //     assert_eq!(blockchain.get_size(), 2);
    //     assert_eq!(blockchain.get_last_header_hash(), hash_block1);
    //
    //     let block2 = BlockHeader {
    //         version: 1,
    //         prev_block_hash: first_hash,
    //         merkle_root_hash: [1; 32],
    //         timestamp: 1234567012,
    //         bits: 0x1d00f0ff,
    //         nonce: 0xacceef,
    //     };
    //     let hash_block2 = block2.hash();
    //     assert!(blockchain.push(block2).is_ok());
    //     assert_eq!(blockchain.get_size(), 2);
    //     assert_eq!(blockchain.get_last_header_hash(), hash_block1);
    //     // assert_eq!(blockchain.heads.len(), 2);
    //
    //     let block3 = BlockHeader {
    //         version: 2,
    //         prev_block_hash: hash_block2,
    //         merkle_root_hash: [0; 32],
    //         timestamp: 1234567890,
    //         bits: 0x1d00ffff,
    //         nonce: 0x1c0644,
    //     };
    //     let hash_block3 = block3.hash();
    //     assert!(blockchain.push(block3).is_ok());
    //     assert_eq!(blockchain.get_size(), 3);
    //     assert_eq!(blockchain.get_last_header_hash(), hash_block3);
    //     // assert_eq!(blockchain.heads.len(), 2);
    //
    //     let block4 = BlockHeader {
    //         version: 2,
    //         prev_block_hash: hash_block3,
    //         merkle_root_hash: [0; 32],
    //         timestamp: 1234567890,
    //         bits: 0x1d00ffff,
    //         nonce: 0x987654,
    //     };
    //     let hash_block4 = block4.hash();
    //     assert!(blockchain.push(block4).is_ok());
    //     assert_eq!(blockchain.get_size(), 4);
    //     assert_eq!(blockchain.get_last_header_hash(), hash_block4);
    //     // assert_eq!(blockchain.heads.len(), 2);
    //
    //     let block5 = BlockHeader {
    //         version: 2,
    //         prev_block_hash: hash_block2,
    //         merkle_root_hash: [0; 32],
    //         timestamp: 1234567890,
    //         bits: 0x1d00ffff,
    //         nonce: 0x123654,
    //     };
    //     // let hash_block5 = block5.hash();
    //     assert!(blockchain.push(block5).is_ok());
    //     assert_eq!(blockchain.get_size(), 4);
    //     assert_eq!(blockchain.get_last_header_hash(), hash_block4);
    //     // assert_eq!(blockchain.heads.len(), 3);
    //     // let largest_head = blockchain.get_largest_head();
    //     // assert_eq!(largest_head.block.nonce, 0x987654);
    //     // assert_eq!(blockchain.heads.len(), 3);
    // }

    #[test]
    fn test_push_invalid_block() {
        let mut blockchain = Blockchain::new();

        let first_hash = blockchain.get_last_header_hash();

        let block = BlockHeader {
            version: 1,
            prev_block_hash: [1; 32],
            merkle_root_hash: [0; 32],
            timestamp: 1234567890,
            bits: 0x1d00ffff,
            nonce: 0xabcdef,
        };
        blockchain.push(block);
        assert_eq!(blockchain.get_size(), 1);
        assert_eq!(blockchain.get_last_header_hash(), first_hash);
        // assert_eq!(blockchain.heads.len(), 1);
    }

    // #[test]
    // fn test_prune_heads() {
    //     let mut blockchain = Blockchain::new();
    //     let first_hash = blockchain.get_last_header_hash();
    //
    //     let mut block: BlockHeader;
    //     let mut hash = first_hash;
    //     for i in 0..80 {
    //         block = BlockHeader {
    //             version: 1,
    //             prev_block_hash: hash,
    //             merkle_root_hash: [i; 32],
    //             timestamp: 1234567890,
    //             bits: 0x1d00ffff,
    //             nonce: 0xabcdef,
    //         };
    //         hash = block.hash();
    //         assert!(blockchain.push(block).is_ok());
    //     }
    //     assert_eq!(blockchain.get_size(), 81);
    //     assert_eq!(blockchain.heads.len(), 1);
    //     let last_header = blockchain.get_last_header_hash();
    //
    //     hash = first_hash;
    //     for i in 0..20 {
    //         block = BlockHeader {
    //             version: 1,
    //             prev_block_hash: hash,
    //             merkle_root_hash: [i; 32],
    //             timestamp: 1234562893,
    //             bits: 0x1d00ff0f,
    //             nonce: 0x1231135,
    //         };
    //         hash = block.hash();
    //         assert!(blockchain.push(block).is_ok());
    //     }
    //     assert_eq!(blockchain.get_size(), 81);
    //     assert_eq!(blockchain.heads.len(), 2);
    //
    //     assert_eq!(blockchain.heads[0].size, 81);
    //     assert_eq!(blockchain.heads[1].size, 21);
    //
    //     assert_eq!(last_header, blockchain.get_last_header_hash());
    //
    //     blockchain.prune_heads();
    //
    //     assert_eq!(blockchain.get_size(), 81);
    //     assert_eq!(blockchain.heads.len(), 1);
    //     assert_eq!(last_header, blockchain.get_last_header_hash());
    // }

    #[test]
    fn testing_utxo_with_one_tx() {
        let mut blockchain = Blockchain::new();
        let first_hash = blockchain.get_last_header_hash();

        let txout1 = TxOut::new(10, vec![]);
        let txout2 = TxOut::new(20, vec![]);
        let txout3 = TxOut::new(3, vec![]);
        let tx1 = RawTransaction::new(vec![], vec![txout1, txout2, txout3]);

        let block1 = BlockHeader {
            version: 1,
            prev_block_hash: first_hash,
            merkle_root_hash: merkle_tree_root(vec![tx1.get_tx_id()]),
            timestamp: 1234567890,
            bits: 0x1d00ffff,
            nonce: 0xabcdef,
        };

        let block_message1 = BlockMessage {
            block_header: block1.clone(),
            txn_count: CompactSize::U8(1),
            txns: vec![tx1],
        };

        assert!(blockchain.push(block1.clone()).is_ok());
        assert!(blockchain.add_block_txs(block_message1).is_ok());
        assert_eq!(blockchain.utxo.len(), 3);

        assert_eq!(blockchain.utxo.get_total_balance(), 33);
    }

    #[test]
    fn testing_spending_utxo_two_tx() {
        let mut blockchain = Blockchain::new();
        let first_hash = blockchain.get_last_header_hash();

        let txout1 = TxOut::new(10, vec![]);
        let txout2 = TxOut::new(20, vec![]);
        let txout3 = TxOut::new(3, vec![]);
        let tx1 = RawTransaction::new(vec![], vec![txout1, txout2, txout3]);

        let outpoint1 = Outpoint::new(tx1.get_tx_id(), 1);
        let outpoint2 = Outpoint::new(tx1.get_tx_id(), 2);
        let tx2in1 = TxIn::new(outpoint1, vec![]);
        let tx2in2 = TxIn::new(outpoint2, vec![]);
        let tx2out1 = TxOut::new(20, vec![]);
        let tx2 = RawTransaction::new(vec![tx2in1, tx2in2], vec![tx2out1]);

        let block1 = BlockHeader {
            version: 1,
            prev_block_hash: first_hash,
            merkle_root_hash: merkle_tree_root(vec![tx1.get_tx_id()]),
            timestamp: 1234567890,
            bits: 0x1d00ffff,
            nonce: 0xabcdef,
        };
        let block_message1 = BlockMessage {
            block_header: block1.clone(),
            txn_count: CompactSize::U8(1),
            txns: vec![tx1],
        };

        let hash_block1 = block1.hash();
        assert!(blockchain.push(block1.clone()).is_ok());
        assert!(blockchain.add_block_txs(block_message1).is_ok());
        assert_eq!(blockchain.utxo.len(), 3);

        let block2 = BlockHeader {
            version: 1,
            prev_block_hash: hash_block1,
            merkle_root_hash: merkle_tree_root(vec![tx2.get_tx_id()]),
            timestamp: 1234577012,
            bits: 0x1d00f0ff,
            nonce: 0xacceef,
        };
        let block_message2 = BlockMessage {
            block_header: block2.clone(),
            txn_count: CompactSize::U8(1),
            txns: vec![tx2],
        };
        assert!(blockchain.push(block2).is_ok());
        assert!(blockchain.add_block_txs(block_message2).is_ok());
        assert_eq!(blockchain.utxo.len(), 2);

        assert_eq!(blockchain.utxo.get_total_balance(), 30);
    }

    #[test]
    fn testing_spending_multiple_txs() {
        let mut blockchain = Blockchain::new();
        let first_hash = blockchain.get_last_header_hash();

        let txout1 = TxOut::new(10, vec![]);
        let txout2 = TxOut::new(5, vec![]);
        let tx1 = RawTransaction::new(vec![], vec![txout1, txout2]);

        let txout3 = TxOut::new(20, vec![]);
        let txout4 = TxOut::new(30, vec![]);
        let tx2 = RawTransaction::new(vec![], vec![txout3, txout4]);

        let outpoint1 = Outpoint::new(tx1.get_tx_id(), 1);
        let outpoint2 = Outpoint::new(tx2.get_tx_id(), 0);
        let tx3in1 = TxIn::new(outpoint1, vec![]);
        let tx3in2 = TxIn::new(outpoint2, vec![]);
        let tx3out1 = TxOut::new(22, vec![]);
        let tx3out2 = TxOut::new(2, vec![]);
        let tx3 = RawTransaction::new(vec![tx3in1, tx3in2], vec![tx3out1, tx3out2]);

        let outpoint3 = Outpoint::new(tx1.get_tx_id(), 0);
        let outpoint4 = Outpoint::new(tx2.get_tx_id(), 1);
        let tx4in1 = TxIn::new(outpoint3, vec![]);
        let tx4in2 = TxIn::new(outpoint4, vec![]);
        let tx4out1 = TxOut::new(40, vec![]);
        let tx4 = RawTransaction::new(vec![tx4in1, tx4in2], vec![tx4out1]);

        let block1 = BlockHeader {
            version: 1,
            prev_block_hash: first_hash,
            merkle_root_hash: merkle_tree_root(vec![tx1.get_tx_id(), tx2.get_tx_id()]),
            timestamp: 1234567890,
            bits: 0x1d00ffff,
            nonce: 0xabcdef,
        };
        let block_message1 = BlockMessage {
            block_header: block1.clone(),
            txn_count: CompactSize::U8(2),
            txns: vec![tx1, tx2],
        };

        let hash_block1 = block1.hash();
        assert!(blockchain.push(block1.clone()).is_ok());
        assert!(blockchain.add_block_txs(block_message1).is_ok());
        assert_eq!(blockchain.utxo.len(), 4);

        let block2 = BlockHeader {
            version: 1,
            prev_block_hash: hash_block1,
            merkle_root_hash: merkle_tree_root(vec![tx3.get_tx_id()]),
            timestamp: 1234577012,
            bits: 0x1d00f0ff,
            nonce: 0xacceef,
        };
        let block_message2 = BlockMessage {
            block_header: block2.clone(),
            txn_count: CompactSize::U8(1),
            txns: vec![tx3],
        };
        let hash_block2 = block2.hash();
        assert!(blockchain.push(block2).is_ok());
        assert!(blockchain.add_block_txs(block_message2).is_ok());
        assert_eq!(blockchain.utxo.len(), 4);

        let block3 = BlockHeader {
            version: 1,
            prev_block_hash: hash_block2,
            merkle_root_hash: merkle_tree_root(vec![tx4.get_tx_id()]),
            timestamp: 1234577012,
            bits: 0x1d00f0ff,
            nonce: 0xacceef,
        };
        let block_message3 = BlockMessage {
            block_header: block3.clone(),
            txn_count: CompactSize::U8(1),
            txns: vec![tx4],
        };
        assert!(blockchain.push(block3).is_ok());
        assert!(blockchain.add_block_txs(block_message3).is_ok());
        assert_eq!(blockchain.utxo.len(), 3);

        assert_eq!(blockchain.utxo.get_total_balance(), 64);
    }

    #[test]
    fn testing_simple_tx_creation() {
        let bitcoin_address = "mgkPm4UebNCJSRGs2Kp2aVE69G8hUEf4d7";
        let private_key = "cSnB7AwCEDKrdq1x2XmHu8f1BHPh6KeuBjeXgssDe2cMpeGDM7oB";

        let mut blockchain = Blockchain::new();
        let first_hash = blockchain.get_last_header_hash();

        let pkhash = &crate::utils::bitcoin_address_to_pkhash(bitcoin_address).unwrap()[..];

        let txout1 = TxOut::new(
            10,
            [&[118, 169, 20], pkhash, &[54, 136, 172]].concat(), // creating P2PKH with public key hash
        );
        let tx1 = RawTransaction::new(vec![], vec![txout1]);
        let tx1_id = tx1.get_tx_id();

        let block1 = BlockHeader {
            version: 1,
            prev_block_hash: first_hash,
            merkle_root_hash: merkle_tree_root(vec![tx1_id]),
            timestamp: 1234567890,
            bits: 0x1d00ffff,
            nonce: 0xabcdef,
        };
        let block_message1 = BlockMessage {
            block_header: block1.clone(),
            txn_count: CompactSize::U8(1),
            txns: vec![tx1],
        };

        let hash_block1 = block1.hash();
        assert!(blockchain.push(block1).is_ok());
        assert!(blockchain.add_block_txs(block_message1).is_ok());
        assert_eq!(blockchain.utxo.len(), 1);

        let out = blockchain.utxo.get(tx1_id, 0).unwrap();
        assert_eq!(out.value, 10);

        println!("TX1 ID {:?}", tx1_id);

        let outs_to_spend = vec![(tx1_id, out)];
        let txout2 = TxOut::new(
            8,
            vec![
                118, 169, 20, 11, 139, 32, 119, 74, 146, 223, 9, 212, 72, 207, 66, 73, 35, 72, 27,
                52, 87, 236, 54, 136, 172,
            ],
        );

        let tx2 = RawTransaction::create_transaction(outs_to_spend, vec![txout2], private_key);
        let tx2_id = tx2.get_tx_id();
        assert!(blockchain.is_valid_tx(&tx2));

        let block2 = BlockHeader {
            version: 1,
            prev_block_hash: hash_block1,
            merkle_root_hash: merkle_tree_root(vec![tx2_id]),
            timestamp: 1234577012,
            bits: 0x1d00f0ff,
            nonce: 0xacceef,
        };
        let block_message2 = BlockMessage {
            block_header: block2.clone(),
            txn_count: CompactSize::U8(1),
            txns: vec![tx2],
        };
        assert!(blockchain.push(block2).is_ok());
        assert!(blockchain.add_block_txs(block_message2).is_ok());
        assert_eq!(blockchain.utxo.len(), 1);

        let out = blockchain.utxo.get(tx2_id, 0).unwrap();
        assert_eq!(out.value, 8);

        assert_eq!(blockchain.utxo.get_total_balance(), 8);
    }

    #[test]
    fn testing_tx_creation_with_multiple_inputs() {
        let bitcoin_address = "mgkPm4UebNCJSRGs2Kp2aVE69G8hUEf4d7";
        let private_key = "cSnB7AwCEDKrdq1x2XmHu8f1BHPh6KeuBjeXgssDe2cMpeGDM7oB";

        let mut blockchain = Blockchain::new();
        let first_hash = blockchain.get_last_header_hash();

        let pkhash = &crate::utils::bitcoin_address_to_pkhash(bitcoin_address).unwrap()[..];

        let txout1 = TxOut::new(
            10,
            [&[118, 169, 20], pkhash, &[54, 136, 172]].concat(), // creating P2PKH with public key hash
        );
        let txout2 = TxOut::new(
            15,
            [&[118, 169, 20], pkhash, &[54, 136, 172]].concat(), // creating P2PKH with public key hash
        );
        let tx1 = RawTransaction::new(vec![], vec![txout1, txout2]);
        let tx1_id = tx1.get_tx_id();

        let block1 = BlockHeader {
            version: 1,
            prev_block_hash: first_hash,
            merkle_root_hash: merkle_tree_root(vec![tx1_id]),
            timestamp: 1234567890,
            bits: 0x1d00ffff,
            nonce: 0xabcdef,
        };
        let block_message1 = BlockMessage {
            block_header: block1.clone(),
            txn_count: CompactSize::U8(1),
            txns: vec![tx1],
        };

        let hash_block1 = block1.hash();
        assert!(blockchain.push(block1).is_ok());
        assert!(blockchain.add_block_txs(block_message1).is_ok());
        assert_eq!(blockchain.utxo.len(), 2);

        let out1 = blockchain.utxo.get(tx1_id, 0).unwrap();
        assert_eq!(out1.value, 10);
        let out2 = blockchain.utxo.get(tx1_id, 1).unwrap();
        assert_eq!(out2.value, 15);

        let outs_to_spend = vec![(tx1_id, out1), (tx1_id, out2)];

        let txout2 = TxOut::new(
            25,
            vec![
                118, 169, 20, 11, 139, 32, 119, 74, 146, 223, 9, 212, 72, 207, 66, 73, 35, 72, 27,
                52, 87, 236, 54, 136, 172,
            ],
        );

        let tx2 = RawTransaction::create_transaction(outs_to_spend, vec![txout2], private_key);
        let tx2_id = tx2.get_tx_id();
        assert!(blockchain.is_valid_tx(&tx2));

        let block2 = BlockHeader {
            version: 1,
            prev_block_hash: hash_block1,
            merkle_root_hash: merkle_tree_root(vec![tx2_id]),
            timestamp: 1234577012,
            bits: 0x1d00f0ff,
            nonce: 0xacceef,
        };
        let block_message2 = BlockMessage {
            block_header: block2.clone(),
            txn_count: CompactSize::U8(1),
            txns: vec![tx2],
        };
        assert!(blockchain.push(block2).is_ok());
        assert!(blockchain.add_block_txs(block_message2).is_ok());
        assert_eq!(blockchain.utxo.len(), 1);

        let out = blockchain.utxo.get(tx2_id, 0).unwrap();
        assert_eq!(out.value, 25);

        assert_eq!(blockchain.utxo.get_total_balance(), 25);
    }
}
