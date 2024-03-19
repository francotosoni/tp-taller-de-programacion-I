use crate::{raw_transaction::Outpoint, script::PubKeyScript};

use super::txs::Txs;
use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct Output {
    pub index: u32,
    pub value: i64,
    pub pkscript: PubKeyScript,
}

impl Output {
    pub fn new(index: u32, value: i64, script_bytes: Vec<u8>) -> Output {
        let pkscript = PubKeyScript::from_bytes(script_bytes);
        Output {
            index,
            value,
            pkscript,
        }
    }
}

#[derive(Debug, Default)]
pub struct UtxoSet {
    pub set: HashMap<[u8; 32], Vec<Output>>,
}

impl UtxoSet {
    pub fn append(&mut self, txs: &Txs) {
        for tx in txs.txns.iter() {
            for (hash, index) in tx.get_inputs() {
                let outputs_opt = self.set.get_mut(&hash);
                let outputs = match outputs_opt {
                    None => continue,
                    Some(i) => i,
                };

                match outputs.iter().position(|x| x.index == index) {
                    None => continue,
                    Some(i) => {
                        outputs.remove(i);
                        if outputs.is_empty() {
                            self.set.remove(&hash);
                        }
                    }
                }
            }
        }

        for tx in txs.txns.iter() {
            self.set.insert(tx.tx_id, tx.tx_out.clone());
        }
    }

    pub fn by_pkhash(&self, pkhash: Vec<u8>) -> Vec<([u8; 32], Output)> {
        let mut outputs = vec![];
        for (hash, outs) in self.set.iter() {
            for o in outs {
                if o.pkscript.can_be_spent_by(&pkhash) {
                    outputs.push((*hash, o.clone()));
                };
            }
        }
        outputs
    }

    pub fn get(&self, hash: [u8; 32], index: u32) -> Option<Output> {
        let tx_utxos = self.set.get(&hash)?;

        for (i, out) in tx_utxos.iter().enumerate() {
            if out.index == index {
                return Some(tx_utxos[i].clone());
            }
        }

        None
    }

    pub fn len(&self) -> usize {
        let mut sum = 0;
        for i in self.set.values() {
            sum += i.len();
        }
        sum
    }

    pub fn is_empty(&self) -> bool {
        if self.len() == 0 {
            return true;
        }
        false
    }

    pub fn get_total_balance(&self) -> i64 {
        let mut sum = 0;
        for txs in self.set.values() {
            for outs in txs {
                sum += outs.value;
            }
        }
        sum
    }

    pub fn get_balance(&self, pkhash: Vec<u8>) -> i64 {
        let mut sum = 0;
        for txs in self.set.values() {
            for output in txs {
                if output.pkscript.can_be_spent_by(&pkhash) {
                    sum += output.value;
                };
            }
        }
        sum
    }

    pub fn get_outpoint_address(&self, previous_output: &Outpoint) -> String {
        let outs = match self.set.get(&previous_output.hash) {
            None => return String::from("Unknown"),
            Some(i) => i,
        };
        if outs.is_empty() {
            return String::from("Unknown");
        }
        for out in outs.iter() {
            if out.index == previous_output.index {
                return out.pkscript.get_address();
            }
        }
        return String::from("Unknown");
    }
}
