use core::fmt;
use std::io::Read;

use crate::protocol_error::ProtocolError;

use super::Serializable;

#[derive(Debug, Clone)]
#[repr(u32)]
pub enum TypeIdentifier {
    MsgTx,
    MsgBlock,
    MsgFilteredBlock,
    MsgCmptBlock,
}

impl TypeIdentifier {
    fn value(&self) -> u32 {
        match *self {
            TypeIdentifier::MsgTx => 1u32,
            TypeIdentifier::MsgBlock => 2u32,
            TypeIdentifier::MsgFilteredBlock => 3u32,
            TypeIdentifier::MsgCmptBlock => 4u32,
        }
    }

    fn from_le_bytes(bytes: [u8; 4]) -> Result<Self, ProtocolError> {
        let identifier: u32 = u32::from_le_bytes(bytes);
        match identifier {
            1 => Ok(TypeIdentifier::MsgTx),
            2 => Ok(TypeIdentifier::MsgBlock),
            3 => Ok(TypeIdentifier::MsgFilteredBlock),
            4 => Ok(TypeIdentifier::MsgCmptBlock),
            _ => Err(ProtocolError::BuildingError(
                "Tipo invalido parseando inventario.".to_string(),
            )),
        }
    }
}

#[derive(Debug)]
pub struct Inventory {
    pub type_identifier: TypeIdentifier,
    pub hash: [u8; 32],
}

impl Inventory {
    pub fn new(type_identifier: TypeIdentifier, hash: [u8; 32]) -> Inventory {
        Inventory {
            type_identifier,
            hash,
        }
    }

    pub fn read_from(stream: &mut dyn Read) -> Result<Inventory, ProtocolError> {
        let mut type_identifier_bytes = [0u8; 4];
        stream.read_exact(&mut type_identifier_bytes)?;

        let mut hash = [0u8; 32];
        stream.read_exact(&mut hash)?;

        let type_identifier = TypeIdentifier::from_le_bytes(type_identifier_bytes)?;

        Ok(Inventory {
            type_identifier,
            hash,
        })
    }
}

impl Serializable for Inventory {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.type_identifier.value().to_le_bytes());
        bytes.extend_from_slice(&(self.hash));

        bytes
    }
}

impl fmt::Display for TypeIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TypeIdentifier::MsgTx => write!(f, "Tx"),
            TypeIdentifier::MsgBlock => write!(f, "Block"),
            TypeIdentifier::MsgFilteredBlock => write!(f, "FilteredBlock"),
            TypeIdentifier::MsgCmptBlock => write!(f, "CmptBlock"),
        }
    }
}
