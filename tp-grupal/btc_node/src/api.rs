use crate::blockchain::txs::Tx;
use crate::protocol_error::ProtocolError;

pub enum NodeApi {
    NewTx(Tx, String, String),
    ConfirmedTx([u8; 32], String),
    Balance(i64, String),
    AddPendingBalance(i64, String),
    AddConfirmedBalance(i64, String),
    PaymentConfirmation(Tx, String, String, i64),
    NodeReady,
    History(Vec<Tx>, String),
    Error(ProtocolError),
    Loading(f64),
    FinishedConnectingToPeers,
}

pub enum WalletApi {
    GetBalance(String),
    GetHistory(String),
    PayTo(String, String, i64, i64),
    AddAddress(String),
}
