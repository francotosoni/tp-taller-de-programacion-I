use crate::{
    log_file::Logger,
    message::{version::VersionMessage, Message},
    protocol_error::ProtocolError,
};

use std::{
    collections::HashMap,
    net::{Ipv6Addr, SocketAddr, TcpStream},
};

#[derive(Debug)]
struct Status {
    _version: VersionMessage,
    stream: TcpStream,
}

#[derive(Debug)]
pub struct Register {
    entries: HashMap<Ipv6Addr, Status>,
    active_nodes: usize,
    logger: Logger,
}

fn to_ipaddr(ip: SocketAddr) -> Ipv6Addr {
    match ip {
        SocketAddr::V4(ipv4_addr) => {
            return ipv4_addr.ip().to_ipv6_mapped();
        }
        SocketAddr::V6(ipv6_addr) => {
            return *ipv6_addr.ip();
        }
    }
}

impl Register {
    pub fn new(filepath: String) -> Register {
        Register {
            entries: HashMap::new(),
            active_nodes: 0,
            logger: Logger::new(filepath),
        }
    }

    pub fn save_connection(
        &mut self,
        stream: TcpStream,
        _version: VersionMessage,
    ) -> Result<(), ProtocolError> {
        let ip = to_ipaddr(stream.peer_addr()?);

        let status = Status { _version, stream };

        self.entries.insert(ip, status);
        self.active_nodes += 1;

        self.logger.log(format!(
            "peer with IP {} is now registered. Handshake completed.",
            ip
        ));

        Ok(())
    }

    pub fn get_n_streams(&self, n: usize) -> Vec<TcpStream> {
        let mut vec: Vec<TcpStream> = vec![];
        for status in self.entries.values() {
            if vec.len() == n {
                return vec;
            }
            let clone = status.stream.try_clone();
            if let Ok(i) = clone {
                vec.push(i);
            }
        }
        vec
    }

    pub fn get_all_streams(&self) -> Vec<TcpStream> {
        self.get_n_streams(self.entries.len())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn log_message(&self, stream: &TcpStream, message: &Message) {
        let ip = match stream.peer_addr() {
            Ok(i) => to_ipaddr(i).to_string(),
            Err(_) => String::from("NONE"),
        };

        self.logger.log(format!("{} sent {}", ip, message));
    }

    pub fn log_error(&self, stream: &TcpStream, error: ProtocolError) {
        let ip = match stream.peer_addr() {
            Ok(i) => to_ipaddr(i).to_string(),
            Err(_) => String::from("NONE"),
        };

        self.logger.log(format!(
            "ERROR handling with message from {}. Error: {}",
            ip, error
        ));
    }
}
