use std::collections::HashMap;

use btc_node::blockchain::txs::Tx;

pub struct Account {
    pub address: String,
    pub wif: String, //private_key
    pub balance: i64,
    pub pending_balance: i64,
    pub transactions: Vec<Tx>,
    pub pending_tx: HashMap<[u8; 32], (Tx, i64, String, String)>,
    pub name: String,
}

impl Account {
    pub fn new(address: String, wif: String, balance: i64, name: String) -> Account {
        Account {
            address,
            wif,
            balance,
            pending_balance: 0,
            transactions: Vec::new(),
            pending_tx: HashMap::new(),
            name,
        }
    }
}
