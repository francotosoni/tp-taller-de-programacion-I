pub mod addr;
pub mod block;
pub mod compact_size;
pub mod fee_filter;
pub mod get_data;
pub mod get_headers;
pub mod headers;
pub mod inv;
pub mod inventory;
pub mod ping;
pub mod pong;
pub mod sendcompact;
pub mod tx;
pub mod version;

use core::fmt;
use std::io::Read;

use bitcoin_hashes::{sha256d, Hash};

use crate::message::{
    addr::AddrMessage, block::BlockMessage, fee_filter::FeeFilterMessage, get_data::GetDataMessage,
    get_headers::GetHeadersMessage, headers::HeadersMessage, inv::InvMessage, ping::PingMessage,
    sendcompact::SendCompactMessage, tx::TxMessage, version::VersionMessage,
};

use crate::utils::bytes_to_hex_string;
use crate::{message_header::MessageHeader, protocol_error::ProtocolError};

#[derive(Debug)]
pub enum Message {
    Ping(PingMessage),
    SendCompact(SendCompactMessage),
    Addr(AddrMessage),
    Block(BlockMessage),
    GetData(GetDataMessage),
    GetHeaders(GetHeadersMessage),
    Headers(HeadersMessage),
    Inv(InvMessage),
    Version(VersionMessage),
    FeeFilter(FeeFilterMessage),
    Tx(TxMessage),
    Mempool,
    Verack,
    SendHeaders,
    UnknownMessage(String),
}

pub trait Serializable {
    fn to_bytes(&self) -> Vec<u8>;
}

fn valid_checksum<T: Serializable>(message: &T, checksum: [u8; 4]) -> bool {
    let bytes = message.to_bytes();
    sha256d::Hash::hash(&bytes).to_byte_array()[0..4] == checksum
}

use crate::constants;

///Message reader instead of message? makes no sense to implement write to to this structure
///it just encapsulates the match from bitcoin node
impl Message {
    pub fn read_from(stream: &mut dyn Read) -> Result<Message, ProtocolError> {
        let header = MessageHeader::read_from(stream)?;
        if header.start_string != constants::START_STRING {
            return Err(ProtocolError::Error(
                "Header's start string is not valid".to_string(),
            ));
        };

        let name = header.command_name()?;

        match &name[..] {
            "sendcmpct" => {
                let send_compact = SendCompactMessage::read_from(stream)?;
                if !valid_checksum(&send_compact, header.checksum) {
                    return Err(ProtocolError::Error("Checksum is not valid".to_string()));
                }

                Ok(Message::SendCompact(send_compact))
            }
            "ping" => {
                let ping = PingMessage::read_from(stream)?;
                if !valid_checksum(&ping, header.checksum) {
                    return Err(ProtocolError::Error("Checksum is not valid".to_string()));
                }

                Ok(Message::Ping(ping))
            }
            "addr" => {
                let addr = AddrMessage::read_from(stream)?;
                if !valid_checksum(&addr, header.checksum) {
                    return Err(ProtocolError::Error("Checksum is not valid".to_string()));
                }

                Ok(Message::Addr(addr))
            }
            "feefilter" => {
                let fee_filter = FeeFilterMessage::read_from(stream)?;
                if !valid_checksum(&fee_filter, header.checksum) {
                    return Err(ProtocolError::Error("Checksum is not valid".to_string()));
                }

                Ok(Message::FeeFilter(fee_filter))
            }
            "getheaders" => {
                let get_headers = GetHeadersMessage::read_from(stream)?;
                if !valid_checksum(&get_headers, header.checksum) {
                    return Err(ProtocolError::Error("Checksum is not valid".to_string()));
                }

                Ok(Message::GetHeaders(get_headers))
            }
            "block" => {
                let block = BlockMessage::read_from(stream)?;
                if !valid_checksum(&block, header.checksum) {
                    return Err(ProtocolError::Error("Checksum is not valid".to_string()));
                }

                Ok(Message::Block(block))
            }
            "headers" => {
                let headers_message = HeadersMessage::read_from(stream)?;
                if !valid_checksum(&headers_message, header.checksum) {
                    return Err(ProtocolError::Error("Checksum is not valid".to_string()));
                }

                Ok(Message::Headers(headers_message))
            }
            "inv" => {
                let inv = InvMessage::read_from(stream)?;
                if !valid_checksum(&inv, header.checksum) {
                    return Err(ProtocolError::Error("Checksum is not valid".to_string()));
                }

                Ok(Message::Inv(inv))
            }
            "sendheaders" => {
                if !header.validate_checksum() {
                    return Err(ProtocolError::Error("Checksum is not valid".to_string()));
                }

                Ok(Message::SendHeaders)
            }
            "mempool" => {
                if !header.validate_checksum() {
                    return Err(ProtocolError::Error("Checksum is not valid".to_string()));
                }

                Ok(Message::Mempool)
            }
            "verack" => {
                if !header.validate_checksum() {
                    return Err(ProtocolError::Error("Checksum is not valid".to_string()));
                }

                Ok(Message::Verack)
            }
            "version" => {
                let version = VersionMessage::read_from(stream)?;
                if !valid_checksum(&version, header.checksum) {
                    return Err(ProtocolError::Error("Checksum is not valid".to_string()));
                }

                Ok(Message::Version(version))
            }
            "tx" => {
                let tx = TxMessage::read_from(stream)?;
                if !valid_checksum(&tx, header.checksum) {
                    return Err(ProtocolError::Error("Checksum is not valid".to_string()));
                }

                Ok(Message::Tx(tx))
            }
            "getdata" => {
                let getdata = GetDataMessage::read_from(stream)?;
                if !valid_checksum(&getdata, header.checksum) {
                    return Err(ProtocolError::Error("Checksum is not valid".to_string()));
                }

                Ok(Message::GetData(getdata))
            }
            _ => Ok(Message::UnknownMessage(name.to_string())),
        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Message::Inv(inv) => write!(f, "INV {}", inv),
            Message::Ping(_) => write!(f, "PING"),
            Message::Addr(_) => write!(f, "ADDR"),
            Message::Verack => write!(f, "VERACK"),
            Message::Version(_) => write!(f, "VERSION"),
            Message::Headers(h) => write!(f, "HEADERS {}", h.count),
            Message::FeeFilter(_) => write!(f, "FEEFILTER"),
            Message::GetHeaders(_) => write!(f, "GETHEADERS"),
            Message::SendCompact(_) => write!(f, "SENDCMPCT"),
            Message::Block(_) => write!(f, "BLOCK"),
            Message::GetData(_) => write!(f, "GETDATA"),
            Message::Mempool => write!(f, "MEMPOOL"),
            Message::SendHeaders => write!(f, "SENDHEADERS"),
            Message::Tx(tx) => write!(f, "TX: {}", bytes_to_hex_string(&tx.tx.get_tx_id()[0..3])),
            Message::UnknownMessage(unknown) => write!(f, "UNKNOWN MESSAGE: {}", unknown),
        }
    }
}
