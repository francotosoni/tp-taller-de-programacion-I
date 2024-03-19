use crate::{
    config::Config, message::compact_size::CompactSize, message_header::MessageHeader,
    protocol_error::ProtocolError,
};
use std::{
    fmt,
    io::{Read, Write},
    net::{Ipv4Addr, Ipv6Addr},
};

pub mod version_message_builder {
    use super::*;
    pub struct VersionMessageBuilder {
        version: Option<i32>,
        services: Option<u64>,
        timestamp: Option<i64>,

        addr_recv_services: Option<u64>,
        addr_recv_ip: Option<Ipv6Addr>,
        addr_recv_port: Option<u16>,

        addr_trans_services: Option<u64>,
        addr_trans_ip: Option<Ipv6Addr>,
        addr_trans_port: Option<u16>,

        nonce: Option<u64>,

        user_agent_bytes: Option<CompactSize>,
        user_agent: Option<Vec<u8>>,

        start_height: Option<i32>,
        relay: Option<u8>,
    }
    impl Default for VersionMessageBuilder {
        fn default() -> Self {
            Self::new()
        }
    }
    impl VersionMessageBuilder {
        pub fn new() -> Self {
            Self {
                version: None,
                services: None,
                timestamp: None,
                addr_recv_services: None,
                addr_recv_ip: None,
                addr_recv_port: None,
                addr_trans_services: None,
                addr_trans_ip: None,
                addr_trans_port: None,
                nonce: None,
                user_agent_bytes: None,
                user_agent: None,
                start_height: None,
                relay: None,
            }
        }

        pub fn version(mut self, version: i32) -> Self {
            self.version = Some(version);
            self
        }

        pub fn services(mut self, services: u64) -> Self {
            self.services = Some(services);
            self
        }

        pub fn timestamp(mut self, timestamp: i64) -> Self {
            self.timestamp = Some(timestamp);
            self
        }

        pub fn addr_recv_services(mut self, addr_recv_services: u64) -> Self {
            self.addr_recv_services = Some(addr_recv_services);
            self
        }

        pub fn addr_recv_ip(mut self, addr_recv_ip: Ipv6Addr) -> Self {
            self.addr_recv_ip = Some(addr_recv_ip);
            self
        }

        pub fn addr_recv_port(mut self, addr_recv_port: u16) -> Self {
            self.addr_recv_port = Some(addr_recv_port);
            self
        }

        pub fn addr_trans_services(mut self, addr_trans_services: u64) -> Self {
            self.addr_trans_services = Some(addr_trans_services);
            self
        }

        pub fn addr_trans_ip(mut self, addr_trans_ip: Ipv6Addr) -> Self {
            self.addr_trans_ip = Some(addr_trans_ip);
            self
        }

        pub fn addr_trans_port(mut self, addr_trans_port: u16) -> Self {
            self.addr_trans_port = Some(addr_trans_port);
            self
        }

        pub fn nonce(mut self, nonce: u64) -> Self {
            self.nonce = Some(nonce);
            self
        }

        pub fn user_agent_bytes(mut self, user_agent_bytes: CompactSize) -> Self {
            self.user_agent_bytes = Some(user_agent_bytes);
            self
        }

        pub fn user_agent(mut self, user_agent: Vec<u8>) -> Self {
            self.user_agent = Some(user_agent);
            self
        }

        pub fn start_height(mut self, start_height: i32) -> Self {
            self.start_height = Some(start_height);
            self
        }

        pub fn relay(mut self, relay: u8) -> Self {
            self.relay = Some(relay);
            self
        }

        pub fn build(self) -> Result<VersionMessage, String> {
            Ok(VersionMessage {
                version: self.version.ok_or("version not set")?,
                services: self.services.ok_or("services not set")?,
                timestamp: self.timestamp.ok_or("timestamp not set")?,
                addr_recv_services: self
                    .addr_recv_services
                    .ok_or("addr_recv_services not set")?,
                addr_recv_ip: self.addr_recv_ip.ok_or("addr_recv_ip not set")?,
                addr_recv_port: self.addr_recv_port.ok_or("addr_recv_port not set")?,
                addr_trans_services: self
                    .addr_trans_services
                    .ok_or("addr_trans_services not set")?,
                addr_trans_ip: self.addr_trans_ip.ok_or("addr_trans_ip not set")?,
                addr_trans_port: self.addr_trans_port.ok_or("addr_trans_port not set")?,
                nonce: self.nonce.ok_or("nonce not set")?,
                user_agent_bytes: self.user_agent_bytes.ok_or("user_agent_bytes not set")?,
                user_agent: self.user_agent.ok_or("user_agent not set")?,
                start_height: self.start_height.ok_or("start_height not set")?,
                relay: self.relay.ok_or("relay not set")?,
            })
        }
    }
}

use chrono::Utc;
use rand::Rng;
use version_message_builder::VersionMessageBuilder;

use super::Serializable;

#[derive(Debug)]
pub struct VersionMessage {
    pub version: i32,
    pub services: u64,
    timestamp: i64,

    addr_recv_services: u64,
    pub addr_recv_ip: Ipv6Addr,
    addr_recv_port: u16,

    addr_trans_services: u64,
    pub addr_trans_ip: Ipv6Addr,
    addr_trans_port: u16,

    nonce: u64,

    user_agent_bytes: CompactSize,
    user_agent: Vec<u8>,

    start_height: i32,
    relay: u8,
}

impl VersionMessage {
    pub fn read_from(stream: &mut dyn Read) -> Result<VersionMessage, ProtocolError> {
        let mut version = [0u8; 4];
        stream.read_exact(&mut version)?;

        let mut services = [0u8; 8];
        stream.read_exact(&mut services)?;

        let mut timestamp = [0u8; 8];
        stream.read_exact(&mut timestamp)?;

        let mut addr_recv_services = [0u8; 8];
        stream.read_exact(&mut addr_recv_services)?;

        let mut addr_recv_ip = [0u8; 16];
        stream.read_exact(&mut addr_recv_ip)?;

        let mut addr_recv_port = [0u8; 2];
        stream.read_exact(&mut addr_recv_port)?;

        let mut addr_trans_services = [0u8; 8];
        stream.read_exact(&mut addr_trans_services)?;

        let mut addr_trans_ip = [0u8; 16];
        stream.read_exact(&mut addr_trans_ip)?;

        let mut addr_trans_port = [0u8; 2];
        stream.read_exact(&mut addr_trans_port)?;

        let mut nonce = [0u8; 8];
        stream.read_exact(&mut nonce)?;

        let user_agent_bytes = CompactSize::read_from(stream)?;
        let mut user_agent = vec![0u8; user_agent_bytes.into_inner()];
        stream.read_exact(&mut user_agent)?;

        let mut start_height = [0u8; 4];
        stream.read_exact(&mut start_height)?;

        let mut relay = [0u8; 1];
        stream.read_exact(&mut relay)?;

        let version_message = VersionMessageBuilder::new()
            .version(i32::from_le_bytes(version))
            .services(u64::from_le_bytes(services))
            .timestamp(i64::from_le_bytes(timestamp))
            .addr_recv_services(u64::from_le_bytes(addr_recv_services))
            .addr_recv_ip(Ipv6Addr::from(u128::from_be_bytes(addr_recv_ip)))
            .addr_recv_port(u16::from_be_bytes(addr_recv_port))
            .addr_trans_services(u64::from_le_bytes(addr_trans_services))
            .addr_trans_ip(Ipv6Addr::from(u128::from_be_bytes(addr_trans_ip)))
            .addr_trans_port(u16::from_be_bytes(addr_trans_port))
            .nonce(u64::from_le_bytes(nonce))
            .user_agent_bytes(user_agent_bytes)
            .user_agent(user_agent)
            .start_height(i32::from_le_bytes(start_height))
            .relay(u8::from_le_bytes(relay))
            .build()?;

        Ok(version_message)
    }

    pub fn write_to(&self, stream: &mut dyn Write) -> Result<(), ProtocolError> {
        let payload = self.to_bytes();

        let header = MessageHeader::new("version".to_string(), payload.clone())?;
        header.write_to(stream)?;

        stream.write_all(&payload[..])?;
        Ok(())
    }

    pub fn new(config: &Config) -> Result<VersionMessage, String> {
        VersionMessageBuilder::new()
            .version(70015)
            .services(0)
            .timestamp(Utc::now().timestamp())
            .addr_recv_services(1)
            .addr_recv_ip(Ipv4Addr::new(127, 0, 0, 1).to_ipv6_mapped())
            .addr_recv_port(18333)
            .addr_trans_services(0)
            .addr_trans_ip(Ipv4Addr::new(127, 0, 0, 1).to_ipv6_mapped())
            .addr_trans_port(config.port)
            .nonce(rand::thread_rng().gen())
            .user_agent_bytes(CompactSize::U8(0))
            .user_agent(Vec::new())
            .start_height(1)
            .relay(1)
            .build()
    }
}

impl Serializable for VersionMessage {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes.extend_from_slice(&self.services.to_le_bytes());
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());

        bytes.extend_from_slice(&self.addr_recv_services.to_le_bytes());
        bytes.extend_from_slice(&self.addr_recv_ip.octets());
        bytes.extend_from_slice(&self.addr_recv_port.to_be_bytes());

        bytes.extend_from_slice(&self.addr_trans_services.to_le_bytes());
        bytes.extend_from_slice(&self.addr_trans_ip.octets());
        bytes.extend_from_slice(&self.addr_trans_port.to_be_bytes());

        bytes.extend_from_slice(&self.nonce.to_le_bytes());

        bytes.extend_from_slice(&self.user_agent_bytes.to_le_bytes());
        bytes.extend_from_slice(&self.user_agent[..]);

        bytes.extend_from_slice(&self.start_height.to_le_bytes());
        bytes.extend_from_slice(&self.relay.to_be_bytes());

        bytes
    }
}

impl fmt::Display for VersionMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "version:: {:?}", self.version)?;
        writeln!(f, "services: {:?}", self.services)?;
        writeln!(f, "timestamp: {:?}", self.timestamp)?;
        writeln!(f, "addr_recv_services: {:?}", self.addr_recv_services)?;
        writeln!(f, "addr_recv_ip: {:?}", self.addr_recv_ip)?;
        writeln!(f, "addr_recv_port: {:?}", self.addr_recv_port)?;
        writeln!(f, "addr_trans_services: {:?}", self.addr_trans_services)?;
        writeln!(f, "addr_trans_ip: {:?}", self.addr_trans_ip)?;
        writeln!(f, "addr_trans_port: {:?}", self.addr_trans_port)?;
        writeln!(f, "nonce: {:?}", self.nonce)?;
        writeln!(f, "user_agent_bytes: {:?}", self.user_agent_bytes)?;
        writeln!(f, "user_agent: {:?}", self.user_agent)?;
        writeln!(f, "start_height: {:?}", self.start_height)?;
        writeln!(f, "relay: {:?}", self.relay)
    }
}
