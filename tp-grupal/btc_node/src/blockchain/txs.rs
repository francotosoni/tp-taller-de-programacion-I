use crate::{
    message::compact_size::CompactSize,
    raw_transaction::{RawTransaction, TxIn, TxOut},
    script::PubKeyScript,
};

use super::utxo_set::Output;

#[derive(Debug, Clone)]
pub struct Txs {
    pub txns: Vec<Tx>,
}

#[derive(Debug, Clone)]
pub struct Tx {
    pub version: i32,
    pub tx_in: Vec<TxIn>,
    pub tx_out: Vec<Output>,
    pub lock_time: u32,
    pub tx_id: [u8; 32],
}

impl Tx {
    pub fn get_inputs(&self) -> Vec<([u8; 32], u32)> {
        let mut inputs = Vec::new();
        for tx in &self.tx_in {
            inputs.push((tx.previous_output.hash, tx.previous_output.index));
        }
        inputs
    }

    pub fn has_pkhash(&self, pkhash: &Vec<u8>) -> bool {
        for output in &self.tx_out {
            if output.pkscript.can_be_spent_by(pkhash) {
                return true;
            };
        }
        false
    }

    pub fn get_tx_value(&self) -> i64 {
        let mut value = 0;

        for out in &self.tx_out {
            value += out.value;
        }

        value
    }

    pub fn from_raw_tx(tx: &RawTransaction) -> Tx {
        let mut outs: Vec<Output> = vec![];
        for (i, out) in tx.tx_out.iter().enumerate() {
            outs.push(Output::new(i as u32, out.value, out.pk_script.clone()));
        }

        Tx {
            version: tx.version,
            tx_in: tx.tx_in.clone(),
            tx_out: outs,
            lock_time: tx.lock_time,
            tx_id: tx.get_tx_id(),
        }
    }

    pub fn to_raw_tx(&self) -> RawTransaction {
        let mut tx_out: Vec<TxOut> = vec![];

        for out in self.tx_out.iter() {
            tx_out.push(TxOut::new(out.value, out.pkscript.to_vec()));
        }

        RawTransaction {
            version: self.version,
            tx_in_count: CompactSize::new_from_usize(self.tx_in.len()),
            tx_in: self.tx_in.clone(),
            tx_out_count: CompactSize::new_from_usize(tx_out.len()),
            tx_out,
            lock_time: self.lock_time,
        }
    }

    pub fn value_payed_to_address(&self, payee_address: &String) -> i64 {
        let mut value = 0;

        for out in &self.tx_out {
            if let Ok(i) =
                PubKeyScript::can_be_spent_by_address(&out.pkscript.to_vec(), &payee_address)
            {
                if i {
                    value += out.value;
                }
            }
        }

        value
    }
}

impl Txs {
    pub fn from_raw_txs(raw_txs: Vec<RawTransaction>) -> Txs {
        let mut txns: Vec<Tx> = vec![];

        for tx in raw_txs {
            txns.push(Tx::from_raw_tx(&tx));
        }

        Txs { txns }
    }

    pub fn to_raw_txs(&self) -> Vec<RawTransaction> {
        let mut txs: Vec<RawTransaction> = vec![];

        for tx in &self.txns {
            txs.push(tx.to_raw_tx());
        }

        txs
    }

    pub fn get_tx_ids(&self) -> Vec<[u8; 32]> {
        let mut txids = vec![];
        for tx in &self.txns {
            txids.push(tx.tx_id);
        }
        txids
    }

    pub fn get_tx(&self, txid: [u8; 32]) -> Option<Tx> {
        for tx in self.txns.iter() {
            if tx.tx_id == txid {
                return Some(tx.clone());
            }
        }
        None
    }

    pub fn get_txs_by_pkhash(&self, pkhash: &Vec<u8>) -> Vec<Tx> {
        let mut vec = vec![];
        for tx in self.txns.iter() {
            if tx.has_pkhash(pkhash) {
                vec.push(tx.clone());
            };
        }
        vec
    }
}
