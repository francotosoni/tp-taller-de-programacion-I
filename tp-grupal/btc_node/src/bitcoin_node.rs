use glib::Sender;

use crate::{
    api::{NodeApi, WalletApi},
    blockchain::{utxo_set::Output, Blockchain},
    config::Config,
    message::{
        addr::AddrMessage, block::BlockMessage, get_data::GetDataMessage,
        get_headers::GetHeadersMessage, inventory::TypeIdentifier, tx::TxMessage,
        version::VersionMessage, Message,
    },
    message_handlers::{handle_handshake_messages, handle_messages},
    message_header::MessageHeader,
    protocol_error::ProtocolError,
    raw_transaction::{RawTransaction, TxOut},
    register::Register,
    script::PubKeyScript,
    utils::wif_to_pkhash,
    wallet_handlers::handle_wallet_messages,
};

use std::{
    collections::HashMap,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs},
    str::FromStr,
    sync::{mpsc::Receiver, Arc, Mutex, RwLock},
    thread::{self, JoinHandle},
};

#[derive(Debug)]
pub struct Node {
    pub config: Config,
    pub version_message: VersionMessage,
    pub register: Arc<RwLock<Register>>,
    pub blockchain: Arc<Mutex<Blockchain>>,
    pub addrs: Vec<Ipv6Addr>,
    pub mempool: Arc<RwLock<HashMap<[u8; 32], RawTransaction>>>,
    pub wallet_txs: Arc<RwLock<HashMap<[u8; 32], String>>>,
    pub wallet_addresses: RwLock<Vec<String>>,
    pub sender: Sender<NodeApi>,
}

impl Node {
    pub fn new(mut config: Config, sender: Sender<NodeApi>) -> Result<Node, ProtocolError> {
        let version_message = VersionMessage::new(&config)?;

        let mut addrs: Vec<Ipv6Addr> = Vec::new();
        if let Some(host) = config.host.clone() {
            addrs.push(Ipv4Addr::from_str(&host).unwrap().to_ipv6_mapped());
            config.max_listen_peers = 1;
            config.block_downloading_threads = 1;
        } else {
            for addr in config.endpoint.to_socket_addrs()? {
                match addr {
                    SocketAddr::V4(ip) => addrs.push(ip.ip().to_ipv6_mapped()),
                    SocketAddr::V6(ip) => addrs.push(ip.ip().to_owned()),
                }
            }
        }

        let blockchain = match Blockchain::read_from_file(config.blockchain_file.clone()) {
            Ok(chain) => chain,
            Err(e) => {
                eprintln!("ERROR READING BLOCKCHAIN FILE: {}", e);
                Blockchain::new()
            }
        };

        let register = Arc::new(RwLock::new(Register::new(config.log_file.clone())));
        let mempool = Arc::new(RwLock::new(HashMap::new()));
        let wallet_txs = Arc::new(RwLock::new(HashMap::new()));
        let wallet_addresses = RwLock::new(Vec::new());

        Ok(Node {
            config,
            version_message,
            register,
            blockchain: Arc::new(Mutex::new(blockchain)),
            addrs,
            mempool,
            wallet_txs,
            wallet_addresses,
            sender,
        })
    }

    /// Performs handshake with all of the nodes and initializes the blockchain
    pub fn initialize(&mut self) -> Result<(), ProtocolError> {
        for addr in self.addrs.clone().iter() {
            if let Err(e) = self.initialize_connection(*addr) {
                eprintln!("Initialization Error: {}", e);
            };
        }

        self.sender
            .send(NodeApi::FinishedConnectingToPeers)
            .unwrap();

        //Send the change label message to the wallet
        let blockchain = self.blockchain.lock()?;

        blockchain
            .save_to_file(self.config.blockchain_file.clone())
            .unwrap_or_else(|e| eprintln!("ERROR SAVING BLOCKCHAIN TO FILE: {}", e));

        drop(blockchain);

        self.multi_threaded_block_download(self.config.block_downloading_threads)?;

        Ok(())
    }

    /// Connects to a peer, performs the handshake and the headers synchronization with it
    fn initialize_connection(&mut self, addr: Ipv6Addr) -> Result<(), ProtocolError> {
        let socket = SocketAddr::new(std::net::IpAddr::V6(addr), 18333);

        let mut stream = TcpStream::connect_timeout(&socket, self.config.tcp_timeout)?;
        stream.set_read_timeout(Some(self.config.tcp_timeout))?;
        stream.set_write_timeout(Some(self.config.tcp_timeout))?;

        println!("\x1b[33m== CONNECTED address: {} ==\x1b[0m", addr);
        let recv_version = self.handshake(&mut stream)?;

        let blockchain = self.blockchain.lock()?;

        let get_headers = GetHeadersMessage::new(blockchain.get_last_header_hash());
        get_headers.write_to(&mut stream)?;
        drop(blockchain);

        handle_handshake_messages(&self.blockchain, &mut stream, &self.register)?;

        self.register
            .write()?
            .save_connection(stream, recv_version)?;

        Ok(())
    }

    /// Creates a thread for every peer and listens for new messages in all of them
    pub fn listen(self, rcv_node: Receiver<WalletApi>) -> Result<(), ProtocolError> {
        let mut streams = self
            .register
            .read()?
            .get_n_streams(self.config.max_listen_peers);

        let mut handlers = vec![];
        let node = Arc::new(self);

        println!("\x1b[33m== LISTENING STREAMS ==\x1b[0m");
        for stream in streams.drain(..) {
            let n = Arc::clone(&node);
            let handle = thread::spawn(move || {
                if let Err(e) = handle_messages(stream, n) {
                    eprintln!("Thread broke: {}", e);
                };
            });
            handlers.push(handle);
        }

        if node.config.host.is_none() {
            let server_handler = node_server_handler(Arc::clone(&node));
            handlers.push(server_handler);
        }

        let n = Arc::clone(&node);
        if let Err(e) = handle_wallet_messages(rcv_node, n) {
            eprintln!("Wallet communication error: {}", e);
        };

        for handle in handlers {
            handle
                .join()
                .map_err(|_| ProtocolError::Error("Joining the handle of thread".to_string()))?;
        }

        Ok(())
    }

    /// It receives a transaction and sends it to every connected peer.
    /// returns the number of peers that received it succesfully.
    pub fn broadcast_transaction(&self, tx: RawTransaction) -> Result<usize, ProtocolError> {
        self.mempool.write()?.insert(tx.get_tx_id(), tx.clone());

        let tx_message = TxMessage::new(tx);
        let streams = self.register.read()?.get_all_streams();

        let mut peers_sent = 0;
        for mut stream in streams {
            if tx_message.write_to(&mut stream).is_ok() {
                peers_sent += 1;
            }
        }

        if peers_sent == 0 {
            return Err(ProtocolError::Error(
                "Couldn't send the tx to any peer".to_string(),
            ));
        }

        Ok(peers_sent)
    }

    pub fn create_transaction(
        &self,
        payer_wif: &str,
        payee_bitcoin_address: &str,
        amount: i64,
        fee: i64,
    ) -> Result<RawTransaction, ProtocolError> {
        let pkhash = wif_to_pkhash(payer_wif)?;
        let (outs_to_spend, sum) = self.get_outs_to_spend(&pkhash, amount + fee)?;

        let mut outputs = vec![TxOut::new(
            amount,
            PubKeyScript::from_address(payee_bitcoin_address)?.to_vec(),
        )];

        if amount + fee < sum {
            outputs.push(TxOut::new(
                sum - amount - fee,
                PubKeyScript::P2PKH(pkhash.to_vec()).to_vec(),
            ));
        }

        let tx = RawTransaction::create_transaction(outs_to_spend, outputs, payer_wif);

        if !self.blockchain.lock()?.is_valid_tx(&tx) {
            return Err(ProtocolError::Error("Transaction is not valid".to_string()));
        };

        Ok(tx)
    }

    /// It downloads all the blocks since the configurable `block_downloading_timestamp` in the number of threads passed as parameters
    fn multi_threaded_block_download(&self, nthreads: usize) -> Result<(), ProtocolError> {
        let hashes_to_download = self
            .blockchain
            .lock()?
            .get_hashes_since(self.config.block_downloading_timestamp);

        let mut streams = self.register.read()?.get_n_streams(nthreads);

        let chunk_size = (hashes_to_download.len() + nthreads - 1) / nthreads;
        let mut results: Vec<_> = hashes_to_download
            .chunks(chunk_size)
            .rev()
            .take(nthreads)
            .collect();

        let loading_state_mutex = Arc::new(RwLock::new(0f64));

        let mut threads: Vec<JoinHandle<Result<Vec<BlockMessage>, ProtocolError>>> = vec![];
        for _ in 0..nthreads {
            let b = streams.pop().unwrap();
            let hashes = results.pop().unwrap().to_vec();
            let l = loading_state_mutex.clone();
            let thread = thread::spawn(move || -> Result<Vec<BlockMessage>, ProtocolError> {
                let n = Node::download_blocks(b, hashes, l)?;
                Ok(n)
            });
            threads.push(thread);
        }

        let mut a: f64 = 0.0;
        while a < 0.98 {
            a = *loading_state_mutex.read()? / hashes_to_download.len() as f64;
            std::thread::sleep(std::time::Duration::from_secs(1));
            self.sender.send(NodeApi::Loading(a)).unwrap();
        }

        let mut blocks = vec![];
        for t in threads {
            blocks.extend_from_slice(&t.join().unwrap()?);
        }

        let mut blockchain = self.blockchain.lock()?;

        for b in blocks {
            blockchain.add_block_txs(b)?;
        }

        Ok(())
    }

    fn download_blocks(
        mut stream: TcpStream,
        hashes: Vec<[u8; 32]>,
        loading_state: Arc<RwLock<f64>>,
    ) -> Result<Vec<BlockMessage>, ProtocolError> {
        stream.set_read_timeout(None)?;
        let mut requested_blocks = hashes.len();
        if requested_blocks == 0 {
            return Ok(vec![]);
        }
        let getdata = GetDataMessage::new(hashes, TypeIdentifier::MsgBlock);
        getdata.write_to(&mut stream)?;

        let mut blocks = vec![];
        while let Ok(i) = stream.peek(&mut [0u8; 1]) {
            if i == 0 {
                break;
            }

            let m = Message::read_from(&mut stream)?;

            dbg!(requested_blocks);
            if let Message::Block(block) = m {
                blocks.push(block);
                requested_blocks -= 1;
                *loading_state.write()? += 1.0;
            }

            if requested_blocks == 0 {
                break;
            }
        }

        Ok(blocks)
    }

    /// It performs the bitcoin protocol handshake and header sync with `stream`
    pub fn handshake(&mut self, stream: &mut TcpStream) -> Result<VersionMessage, ProtocolError> {
        self.version_message.write_to(stream)?;

        let recv_version_message = match Message::read_from(stream)? {
            Message::Version(v) => v,
            _ => return Err(ProtocolError::Error("Expected version message".to_string())),
        };

        let verack = MessageHeader::new("verack".to_string(), Vec::new())?;
        verack.write_to(stream)?;

        match Message::read_from(stream)? {
            Message::Verack => {}
            _ => return Err(ProtocolError::Error("Expected verack message".to_string())),
        };

        Ok(recv_version_message)
    }

    fn _get_addresses(&self, stream: &mut TcpStream) -> Result<Vec<Ipv6Addr>, ProtocolError> {
        let getaddr = MessageHeader::new("getaddr".to_string(), Vec::new())?;
        getaddr.write_to(stream)?;

        let addr_message = AddrMessage::read_from(stream)?;

        let mut ips: Vec<Ipv6Addr> = vec![];
        for i in addr_message.ip_addresses {
            ips.push(i.ip);
        }

        Ok(ips)
    }

    fn get_outs_to_spend(
        &self,
        pkhash: &[u8; 20],
        amount: i64,
    ) -> Result<(Vec<([u8; 32], Output)>, i64), ProtocolError> {
        let mut utxo = self.blockchain.lock()?.get_utxo(pkhash.to_vec());
        utxo.sort_by(|a, b| b.1.value.partial_cmp(&a.1.value).unwrap());

        let mut out_to_spend = vec![];
        let mut sum = 0;
        for output in utxo {
            sum += output.1.value;
            out_to_spend.push(output);
            if sum >= amount {
                break;
            }
        }

        if sum < amount {
            return Err(ProtocolError::Error("Insufficient balance".to_string()));
        }

        Ok((out_to_spend, sum))
    }
}

fn node_server_handler(node: Arc<Node>) -> std::thread::JoinHandle<()> {
    thread::spawn(move || {
        let listener = TcpListener::bind("127.0.0.1:18333").unwrap();
        println!("\x1b[33m== LISTENING FOR NEW CONNECTIONS IN PORT 18333 ==\x1b[0m");

        let mut handlers = vec![];
        for stream in listener.incoming() {
            println!("NEW CONNECTION");
            let n = Arc::clone(&node);
            let mut stream = stream.unwrap();
            let handle = thread::spawn(move || -> Result<(), ProtocolError> {
                match Message::read_from(&mut stream)? {
                    Message::Version(_) => {}
                    _ => return Err(ProtocolError::Error("Expected version message".to_string())),
                };

                n.version_message.write_to(&mut stream)?;

                match Message::read_from(&mut stream)? {
                    Message::Verack => {}
                    _ => return Err(ProtocolError::Error("Expected verack message".to_string())),
                };

                let verack = MessageHeader::new("verack".to_string(), Vec::new())?;
                verack.write_to(&mut stream).unwrap();

                if let Err(e) = handle_messages(stream, n) {
                    eprintln!("Thread broke: {}", e);
                };
                Ok(())
            });
            handlers.push(handle)
        }

        for handle in handlers {
            handle
                .join()
                .map_err(|_| ProtocolError::Error("Joining the handle of thread".to_string()))
                .unwrap()
                .unwrap();
        }
    })
}
