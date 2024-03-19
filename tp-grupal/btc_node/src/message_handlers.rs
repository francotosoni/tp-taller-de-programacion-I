use std::{
    collections::HashMap,
    io::Write,
    net::TcpStream,
    sync::{Arc, Mutex, RwLock},
};

use crate::{
    api::NodeApi,
    bitcoin_node::Node,
    blockchain::{txs::Tx, Blockchain},
    message::{
        block::BlockMessage,
        compact_size::CompactSize,
        get_data::GetDataMessage,
        get_headers::GetHeadersMessage,
        headers::HeadersMessage,
        inv::InvMessage,
        inventory::{Inventory, TypeIdentifier},
        pong::PongMessage,
        tx::TxMessage,
        Message,
    },
    message_header::MessageHeader,
    protocol_error::ProtocolError,
    raw_transaction::RawTransaction,
    register::Register,
    script::PubKeyScript,
};

pub fn handle_handshake_messages(
    blockchain: &Arc<Mutex<Blockchain>>,
    stream: &mut TcpStream,
    register: &Arc<RwLock<Register>>,
) -> Result<(), ProtocolError> {
    let mut pings_available = 2;
    loop {
        let m = Message::read_from(stream)?;

        register.read()?.log_message(stream, &m);

        match m {
            Message::Headers(msg) => {
                let size = handle_headers(blockchain, stream, msg)?;
                if size < 2000 {
                    break;
                }
            }
            Message::Ping(ping) => {
                let pong = PongMessage::new(ping.get_nonce());
                pong.write_to(stream)?;
                pings_available -= 1;
                if pings_available == 0 {
                    break;
                }
            }
            _ => {}
        }
    }

    Ok(())
}

pub fn handle_messages(mut stream: TcpStream, node: Arc<Node>) -> Result<(), ProtocolError> {
    stream.set_read_timeout(None)?;
    if let Ok(m) = MessageHeader::new("mempool".to_string(), vec![]) {
        if m.write_to(&mut stream).is_err() {
            eprintln!("Error sending mempool message");
        };
    }

    loop {
        let m = match Message::read_from(&mut stream) {
            Err(_) => continue,
            Ok(m) => m,
        };

        if let Ok(r) = node.register.write() {
            r.log_message(&stream, &m);
        };

        let res: Result<(), ProtocolError> = match m {
            Message::Headers(h) => handle_headers(&node.blockchain, &mut stream, h).map(|_| ()),
            Message::GetData(g) => handle_get_data(g, &node.mempool, &mut stream, &node.blockchain),
            Message::Ping(ping) => PongMessage::new(ping.get_nonce()).write_to(&mut stream),
            Message::Inv(inv) => handle_inv(inv, &node.mempool, &mut stream),
            Message::Block(block) => handle_block(&node, block),
            Message::Tx(tx_msg) => handle_tx(&node, tx_msg),
            Message::GetHeaders(gh) => handle_get_headers(gh, &node.blockchain, &mut stream),
            Message::Mempool => handle_mempool(&node.mempool, &mut stream),
            _ => Ok(()),
        };

        if let Err(e) = res {
            if let Ok(r) = node.register.write() {
                r.log_error(&stream, e);
            };
        };
    }
}

fn handle_mempool(
    mempool: &RwLock<HashMap<[u8; 32], RawTransaction>>,
    stream: &mut TcpStream,
) -> Result<(), ProtocolError> {
    let mut inventory = vec![];
    for hash in mempool.read()?.keys() {
        inventory.push(Inventory::new(TypeIdentifier::MsgTx, hash.clone()));
    }
    let inv_message = InvMessage {
        count: CompactSize::new_from_usize(inventory.len()),
        inventory,
    };
    inv_message.write_to(stream)
}

fn handle_get_headers(
    getheaders: GetHeadersMessage,
    blockchain: &Arc<Mutex<Blockchain>>,
    stream: &mut TcpStream,
) -> Result<(), ProtocolError> {
    if getheaders.block_header_hashes.len() == 0 {
        return Ok(());
    }
    let headers = blockchain
        .lock()?
        .get_headers(getheaders.block_header_hashes[0]);
    HeadersMessage::new(headers).write_to(stream)
}

fn handle_tx(node: &Arc<Node>, tx_msg: TxMessage) -> Result<(), ProtocolError> {
    let txid = tx_msg.tx.get_tx_id();
    if node.mempool.read()?.contains_key(&txid) {
        return Ok(());
    } else {
        node.mempool.write()?.insert(txid, tx_msg.tx.clone());
        if let Err(e) = node.broadcast_transaction(tx_msg.tx.clone()) {
            eprintln!("Couldn't re-broadcast the transaction: {:?}", e);
        };
    };

    let tx = tx_msg.tx;
    let addresses = node.wallet_addresses.read()?;

    for addr in addresses.iter() {
        let mut is_spent = false;
        for out in &tx.tx_out {
            if PubKeyScript::can_be_spent_by_address(&out.pk_script, addr)? {
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
                .send(crate::api::NodeApi::NewTx(
                    transaction,
                    payer_addr,
                    addr.to_string(),
                ))
                .map_err(|_| ProtocolError::Error("Wallet sender error".to_string()))?;

            node.wallet_txs.write()?.insert(txid, addr.to_string());
        }
    }

    Ok(())
}

fn handle_headers(
    blockchain: &Arc<Mutex<Blockchain>>,
    stream: &mut dyn Write,
    mut msg: HeadersMessage,
) -> Result<usize, ProtocolError> {
    let mut blockchain = blockchain.lock()?;

    for query in msg.headers.drain(..) {
        (*blockchain).push(query)?
    }

    if msg.count.into_inner() == 2000 {
        let get_headers = GetHeadersMessage::new((*blockchain).get_last_header_hash());
        get_headers.write_to(stream)?;
    }

    Ok(msg.count.into_inner())
}

fn handle_get_data(
    getdata: GetDataMessage,
    mempool: &Arc<RwLock<HashMap<[u8; 32], RawTransaction>>>,
    stream: &mut TcpStream,
    blockchain: &Arc<Mutex<Blockchain>>,
) -> Result<(), ProtocolError> {
    let mut requested_blocks = vec![];
    for inv in getdata.inventory {
        match inv.type_identifier {
            TypeIdentifier::MsgTx => {
                let m = mempool.read()?;
                if let Some(tx) = m.get(&inv.hash) {
                    TxMessage::new(tx.clone()).write_to(stream)?;
                };
            }
            TypeIdentifier::MsgBlock => requested_blocks.push(inv.hash),
            TypeIdentifier::MsgCmptBlock => {}
            TypeIdentifier::MsgFilteredBlock => {}
        }
    }

    if !requested_blocks.is_empty() {
        for block_message in blockchain.lock()?.get_blocks(requested_blocks) {
            block_message.write_to(stream)?;
        }
    }

    Ok(())
}

fn handle_inv(
    inv: InvMessage,
    mempool: &Arc<RwLock<HashMap<[u8; 32], RawTransaction>>>,
    stream: &mut TcpStream,
) -> Result<(), ProtocolError> {
    let mut to_request: Vec<Inventory> = vec![];

    for inv in inv.inventory {
        match inv.type_identifier {
            TypeIdentifier::MsgTx => {
                if !mempool.read()?.contains_key(&inv.hash) {
                    to_request.push(Inventory::new(inv.type_identifier, inv.hash));
                };
            }
            TypeIdentifier::MsgBlock => {
                to_request.push(Inventory::new(inv.type_identifier, inv.hash));
            }
            _ => {}
        }
    }

    if !to_request.is_empty() {
        return GetDataMessage::new_from_inventory(to_request).write_to(stream);
    };

    Ok(())
}

fn handle_block(node: &Arc<Node>, block_msg: BlockMessage) -> Result<(), ProtocolError> {
    println!("HANDLE BLOCK");
    let block = node.blockchain.lock()?.push_full_block(block_msg)?;

    let mut wallet_tx = node.wallet_txs.write()?;
    let mut mempool = node.mempool.write()?;

    let txs = block.txs.clone().unwrap().txns;

    for tx in txs {
        if wallet_tx.contains_key(&tx.tx_id) {
            let addr = match wallet_tx.remove(&tx.tx_id) {
                None => {
                    return Err(ProtocolError::Error(
                        "Error getting tx from wallet transactions".to_string(),
                    ))
                }
                Some(i) => i,
            };

            node.sender
                .send(NodeApi::ConfirmedTx(tx.tx_id, addr))
                .map_err(|_| ProtocolError::Error("Wallet sender error".to_string()))?;

            let addresses = node.wallet_addresses.read()?;
            for addr in addresses.iter() {
                for out in &tx.tx_out {
                    if PubKeyScript::can_be_spent_by_address(&out.pkscript.to_vec(), addr)? {
                        node.sender
                            .send(NodeApi::AddConfirmedBalance(out.value, addr.to_string()))
                            .map_err(|_| ProtocolError::Error("Wallet sender error".to_string()))?;
                    };
                }
            }

            return Ok(());
        }

        if mempool.contains_key(&tx.tx_id) {
            mempool.remove(&tx.tx_id);
        }
    }

    Ok(())
}
