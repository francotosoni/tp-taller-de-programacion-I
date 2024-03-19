use crate::{
    api::{NodeApi, WalletApi},
    bitcoin_node::Node,
    blockchain::txs::Tx,
    protocol_error::ProtocolError,
    script::PubKeyScript,
};
use std::sync::{mpsc::Receiver, Arc};

pub fn handle_wallet_messages(
    rx: Receiver<WalletApi>,
    node: Arc<Node>,
) -> Result<(), ProtocolError> {
    node.sender
        .send(NodeApi::NodeReady)
        .map_err(|_| ProtocolError::Error("Wallet sender error".to_string()))?;

    for msg in rx {
        let res = match msg {
            WalletApi::GetBalance(addr) => get_balance(addr, &node),
            WalletApi::GetHistory(addr) => get_history(addr, &node),
            WalletApi::PayTo(wif, addr, amount, fee) => pay_to(wif, addr, amount, fee, &node),
            WalletApi::AddAddress(addr) => add_address(addr, &node),
        };

        if let Err(e) = res {
            node.sender
                .send(NodeApi::Error(e))
                .map_err(|_| ProtocolError::Error("Wallet sender error".to_string()))?;
        }
    }
    Ok(())
}

fn get_balance(addr: String, node: &Arc<Node>) -> Result<(), ProtocolError> {
    let pkhash = crate::utils::bitcoin_address_to_pkhash(&addr)?;
    let balance = node.blockchain.lock()?.utxo.get_balance(pkhash);
    node.sender
        .send(NodeApi::Balance(balance, addr))
        .map_err(|_| ProtocolError::Error("Wallet sender error".to_string()))?;
    Ok(())
}

fn get_history(addr: String, node: &Arc<Node>) -> Result<(), ProtocolError> {
    let pkhash = crate::utils::bitcoin_address_to_pkhash(&addr)?;
    let history = node.blockchain.lock()?.get_tx_history(pkhash);
    node.sender
        .send(NodeApi::History(history, addr))
        .map_err(|_| ProtocolError::Error("Wallet sender error".to_string()))?;
    Ok(())
}

fn pay_to(
    wif: String,
    addr: String,
    amount: i64,
    fee: i64,
    node: &Arc<Node>,
) -> Result<(), ProtocolError> {
    let payer_address = crate::utils::wif_to_bitcoin_address(&wif);
    let tx = node.create_transaction(&wif, &addr, amount, fee)?;
    node.wallet_txs
        .write()?
        .insert(tx.get_tx_id(), payer_address.clone());

    node.broadcast_transaction(tx.clone())?;
    node.sender
        .send(NodeApi::PaymentConfirmation(
            Tx::from_raw_tx(&tx),
            payer_address,
            addr,
            amount,
        ))
        .map_err(|_| ProtocolError::Error("Wallet sender error".to_string()))?;

    let addresses = node.wallet_addresses.read()?;
    for addr in addresses.iter() {
        for out in &tx.tx_out {
            if PubKeyScript::can_be_spent_by_address(&out.pk_script, addr)? {
                node.sender
                    .send(NodeApi::AddPendingBalance(out.value, addr.to_string()))
                    .map_err(|_| ProtocolError::Error("Wallet sender error".to_string()))?;
            };
        }
    }

    Ok(())
}

fn add_address(addr: String, node: &Arc<Node>) -> Result<(), ProtocolError> {
    node.wallet_addresses.write()?.push(addr.clone());
    let pkhash = crate::utils::bitcoin_address_to_pkhash(&addr)?;
    let chain = node.blockchain.lock()?;

    let history = chain.get_tx_history(pkhash.clone());
    let balance = chain.utxo.get_balance(pkhash);

    node.sender
        .send(NodeApi::Balance(balance, addr.clone()))
        .map_err(|_| ProtocolError::Error("Wallet sender error".to_string()))?;

    node.sender
        .send(NodeApi::History(history, addr.clone()))
        .map_err(|_| ProtocolError::Error("Wallet sender error".to_string()))?;

    let mempool = node.mempool.read()?;
    for tx in mempool.values() {
        let mut is_spent = false;
        for out in &tx.tx_out {
            if PubKeyScript::can_be_spent_by_address(&out.pk_script, &addr)? {
                node.sender
                    .send(NodeApi::AddPendingBalance(out.value, addr.to_string()))
                    .map_err(|_| ProtocolError::Error("Wallet sender error".to_string()))?;

                is_spent = true;
            };
        }

        if is_spent {
            let transaction = Tx::from_raw_tx(&tx);

            let payer_addr = node
                .blockchain
                .lock()?
                .utxo
                .get_outpoint_address(&transaction.tx_in[0].previous_output);

            node.sender
                .send(NodeApi::NewTx(transaction, payer_addr, addr.to_string()))
                .map_err(|_| ProtocolError::Error("Wallet sender error".to_string()))?;

            node.wallet_txs
                .write()?
                .insert(tx.get_tx_id(), addr.to_string());
        }
    }

    Ok(())
}
