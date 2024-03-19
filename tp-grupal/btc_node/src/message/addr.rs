use std::{io::Read, net::Ipv6Addr};

use crate::{message::compact_size::CompactSize, protocol_error::ProtocolError};

use super::Serializable;

#[derive(Debug)]
pub struct NetworkAddr {
    time: u32,
    services: u64,
    pub ip: Ipv6Addr,
    port: u16,
}

impl NetworkAddr {
    pub fn read_from(stream: &mut dyn Read) -> Result<NetworkAddr, ProtocolError> {
        let mut time_bytes = [0u8; 4];
        stream.read_exact(&mut time_bytes)?;

        let mut services_bytes = [0u8; 8];
        stream.read_exact(&mut services_bytes)?;

        let mut ip_bytes = [0u8; 16];
        stream.read_exact(&mut ip_bytes)?;

        let mut port_bytes = [0u8; 2];
        stream.read_exact(&mut port_bytes)?;

        Ok(NetworkAddr {
            time: u32::from_le_bytes(time_bytes),
            services: u64::from_le_bytes(services_bytes),
            ip: Ipv6Addr::from(u128::from_be_bytes(ip_bytes)),
            port: u16::from_be_bytes(port_bytes),
        })
    }
}

impl Serializable for NetworkAddr {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.time.to_le_bytes());
        bytes.extend_from_slice(&self.services.to_le_bytes());
        bytes.extend_from_slice(&self.ip.octets());
        bytes.extend_from_slice(&self.port.to_be_bytes());

        bytes
    }
}

#[derive(Debug)]
pub struct AddrMessage {
    count: CompactSize,
    pub ip_addresses: Vec<NetworkAddr>,
}

impl AddrMessage {
    pub fn read_from(stream: &mut dyn Read) -> Result<AddrMessage, ProtocolError> {
        let count = CompactSize::read_from(stream)?;

        let mut ip_addresses: Vec<NetworkAddr> = Vec::new();

        for _ in 0..count.into_inner() {
            ip_addresses.push(NetworkAddr::read_from(stream)?);
        }

        Ok(AddrMessage {
            count,
            ip_addresses,
        })
    }
}

impl Serializable for AddrMessage {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.count.to_le_bytes());
        for ip in &self.ip_addresses {
            bytes.extend_from_slice(&ip.to_bytes());
        }

        bytes
    }
}
