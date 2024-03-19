use bitcoin_hashes::{sha256d, Hash};
use secp256k1::{ecdsa, Message, PublicKey, Secp256k1};

use crate::{
    constants::{P2PKH_BYTE, P2SH_BYTE, SIGHASH_ALL},
    protocol_error::ProtocolError,
    raw_transaction::RawTransaction,
    utils::hash160,
};

#[derive(Debug, Default, Clone)]
pub enum PubKeyScript {
    P2PKH(Vec<u8>),
    P2SH(Vec<u8>),
    SCRIPT(Vec<u8>),
    #[default]
    EMPTY,
}

const OP_EQUAL: u8 = 135;
const OP_EQUALVERIFY: u8 = 136;
const OP_DUP: u8 = 118;
const OP_HASH160: u8 = 169;
const OP_CHECKSIG: u8 = 172;
const OP_HASH256: u8 = 170;
const OP_0: u8 = 0;
const OP_1: u8 = 1;

impl PubKeyScript {
    pub fn from_bytes(bytes: Vec<u8>) -> PubKeyScript {
        if bytes.len() < 3 {
            return Self::SCRIPT(bytes);
        }
        match &bytes[..] {
            [OP_DUP, OP_HASH160, 20, .., OP_EQUALVERIFY, OP_CHECKSIG] => {
                let pkhash = &bytes[3..23];
                PubKeyScript::P2PKH(pkhash.to_vec())
            }
            [OP_HASH160, 20, .., OP_EQUAL] => {
                let reedeem_script_hash = &bytes[2..22];
                PubKeyScript::P2SH(reedeem_script_hash.to_vec())
            }
            _ => PubKeyScript::SCRIPT(bytes),
        }
    }

    pub fn from_address(address: &str) -> Result<PubKeyScript, ProtocolError> {
        let address_decoded = bs58::decode(address)
            .into_vec()
            .map_err(|_| ProtocolError::Error("Error parsing base 58".to_string()))?;

        let len = address_decoded.len();
        let checksum = &address_decoded[(len - 4)..len];
        let check = &sha256d::Hash::hash(&address_decoded[..(len - 4)]).to_byte_array()[0..4];

        if check != checksum {
            return Err(ProtocolError::Error("Invalid address checksum".to_string()));
        }

        match address_decoded[0] {
            P2PKH_BYTE => Ok(PubKeyScript::P2PKH(address_decoded[1..21].to_vec())),
            P2SH_BYTE => Ok(PubKeyScript::P2SH(address_decoded[1..21].to_vec())),
            _ => Ok(PubKeyScript::SCRIPT(address_decoded[1..21].to_vec())),
        }
    }

    pub fn can_be_spent_by(&self, hash: &Vec<u8>) -> bool {
        match &self {
            PubKeyScript::P2PKH(a) => a == hash,
            PubKeyScript::P2SH(a) => a == hash,
            _ => false,
        }
    }

    pub fn can_be_spent_by_address(
        script: &Vec<u8>,
        address: &String,
    ) -> Result<bool, ProtocolError> {
        let address_decoded = bs58::decode(address)
            .into_vec()
            .map_err(|_| ProtocolError::Error("Error parsing address base 58".to_string()))?;

        if address_decoded.len() < 22 {
            return Err(ProtocolError::Error("Address is invalid".to_string()));
        };

        let hash = address_decoded[1..21].to_vec();
        let s = PubKeyScript::from_bytes(script.to_vec());
        match s {
            PubKeyScript::P2PKH(a) => Ok(a == hash),
            PubKeyScript::P2SH(a) => Ok(a == hash),
            _ => Ok(false),
        }
    }

    pub fn evaluate(&self, tx: RawTransaction, index: usize) -> bool {
        if index >= tx.tx_in.len() {
            return false;
        }

        match self {
            PubKeyScript::P2PKH(_) => evaluate_script(self.to_vec(), tx, index),
            _ => false,
        }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        match self {
            PubKeyScript::P2PKH(pk) => [
                &[OP_DUP, OP_HASH160, 20],
                &pk[..],
                &[OP_EQUALVERIFY, OP_CHECKSIG],
            ]
            .concat(),
            PubKeyScript::P2SH(rhash) => [&[OP_HASH160, 20], &rhash[..], &[OP_EQUAL]].concat(),
            PubKeyScript::SCRIPT(b) => b.to_vec(),
            _ => vec![],
        }
    }

    pub fn get_address(&self) -> String {
        match self {
            PubKeyScript::P2PKH(pkhash) => {
                let mut addr = [&[P2PKH_BYTE], &pkhash[..]].concat();
                let checksum = &sha256d::Hash::hash(&addr).to_byte_array()[0..4];
                addr.extend_from_slice(checksum);
                bs58::encode(addr).into_string()
            }
            PubKeyScript::P2SH(pkhash) => {
                let mut addr = [&[P2SH_BYTE], &pkhash[..]].concat();
                let checksum = &sha256d::Hash::hash(&addr).to_byte_array()[0..4];
                addr.extend_from_slice(checksum);
                bs58::encode(addr).into_string()
            }
            _ => String::from("Unknown"),
        }
    }
}

fn evaluate_script(pubkey_script: Vec<u8>, tx: RawTransaction, input: usize) -> bool {
    let mut stack: Vec<Vec<u8>> = vec![];
    let mut script = pubkey_script.clone();
    script.reverse();
    let mut tmp = tx.tx_in[input].signature_script.clone();
    tmp.reverse();
    script.extend_from_slice(&tmp[..]);

    while let Some(op) = script.pop() {
        match op {
            1..=75 => {
                let mut v: Vec<u8> = vec![];
                for _ in 0..op {
                    v.push(script.pop().unwrap());
                }
                stack.push(v);
            }
            OP_DUP => {
                let a = stack.len();
                if a < 1 {
                    return false;
                }
                stack.push(stack[stack.len() - 1].clone());
            }
            OP_HASH160 => {
                match stack.pop() {
                    None => return false,
                    Some(h) => stack.push(hash160(&h).to_vec()),
                };
            }
            OP_EQUAL => {
                if stack.len() < 2 {
                    return false;
                }
                let a = stack.pop().unwrap();
                let b = stack.pop().unwrap();

                if a != b {
                    stack.push(vec![OP_1]);
                } else {
                    stack.push(vec![OP_0]);
                }
            }
            OP_EQUALVERIFY => {
                if stack.len() < 2 {
                    return false;
                }
                let a = stack.pop().unwrap();
                let b = stack.pop().unwrap();

                if a != b {
                    return false;
                }
            }
            OP_CHECKSIG => {
                if stack.len() < 2 {
                    return false;
                }
                let pk = stack.pop().unwrap();
                let mut signature = stack.pop().unwrap();
                if let Some(flag) = signature.pop() {
                    if flag != SIGHASH_ALL {
                        return false;
                    }
                } else {
                    return false;
                };

                let serialization =
                    sha256d::Hash::hash(&tx.serialize(input, pubkey_script)).to_byte_array();

                let secp = Secp256k1::verification_only();
                let m = Message::from_slice(&serialization).unwrap();
                let s = ecdsa::Signature::from_der(&signature).unwrap();
                let p = PublicKey::from_slice(&pk).unwrap();

                if secp.verify_ecdsa(&m, &s, &p).is_ok() {
                    return true;
                } else {
                    return false;
                };
            }
            OP_HASH256 => {
                let h = match stack.pop() {
                    None => return false,
                    Some(i) => i,
                };
                let hash = sha256d::Hash::hash(&h).to_byte_array();
                stack.push(hash.to_vec());
            }
            _ => return false,
        }
    }

    if let Some(a) = stack.last() {
        a != &vec![OP_0]
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::compact_size::CompactSize;
    use crate::raw_transaction::{Outpoint, RawTransaction, TxIn, TxOut};

    #[test]
    fn test_p2pkh_script() {
        // a7ddc873151911bf9fc49d8094e11cc2412d6d27a7670478be8ab7a4a55d2a4d
        let prev_tx = RawTransaction {
            version: 2,
            tx_in_count: CompactSize::U8(1),
            tx_in: vec![TxIn {
                previous_output: Outpoint {
                    hash: [
                        142, 3, 13, 148, 34, 150, 166, 86, 255, 236, 84, 140, 134, 241, 133, 33,
                        197, 121, 24, 180, 3, 155, 121, 116, 119, 15, 1, 65, 174, 38, 225, 54,
                    ],
                    index: 1,
                },
                script_bytes: CompactSize::U8(106),
                signature_script: vec![
                    71, 48, 68, 2, 32, 19, 123, 1, 253, 78, 60, 245, 141, 144, 191, 228, 151, 3,
                    107, 186, 184, 170, 60, 42, 96, 229, 124, 48, 17, 30, 223, 237, 252, 154, 230,
                    194, 144, 2, 32, 85, 16, 245, 214, 205, 118, 70, 80, 156, 95, 116, 34, 102,
                    130, 12, 204, 30, 174, 255, 57, 10, 218, 206, 95, 77, 53, 183, 175, 213, 225,
                    129, 193, 1, 33, 3, 78, 168, 32, 115, 178, 1, 88, 105, 240, 192, 59, 157, 180,
                    84, 172, 133, 83, 45, 253, 220, 139, 216, 187, 206, 142, 162, 135, 248, 150,
                    82, 135, 55,
                ],
                sequence: 4294967293,
            }],
            tx_out_count: CompactSize::U8(2),
            tx_out: vec![
                TxOut {
                    value: 1640823,
                    pk_script_bytes: CompactSize::U8(25),
                    pk_script: vec![
                        118, 169, 20, 120, 194, 32, 54, 33, 183, 126, 69, 245, 233, 15, 86, 117,
                        121, 153, 198, 78, 93, 123, 113, 136, 172,
                    ],
                },
                TxOut {
                    value: 6850421802,
                    pk_script_bytes: CompactSize::U8(25),
                    pk_script: vec![
                        118, 169, 20, 133, 244, 60, 190, 242, 23, 3, 209, 177, 89, 20, 54, 126,
                        144, 131, 228, 245, 227, 12, 247, 136, 172,
                    ],
                },
            ],
            lock_time: 2437013,
        };

        // bded888b7d146268e17fb590250bfb411545296d205ce6b5667a38f18c0e010b
        let curr_tx = RawTransaction {
            version: 2,
            tx_in_count: CompactSize::U8(1),
            tx_in: vec![TxIn {
                previous_output: Outpoint {
                    hash: [
                        77, 42, 93, 165, 164, 183, 138, 190, 120, 4, 103, 167, 39, 109, 45, 65,
                        194, 28, 225, 148, 128, 157, 196, 159, 191, 17, 25, 21, 115, 200, 221, 167,
                    ],
                    index: 1,
                },
                script_bytes: CompactSize::U8(106),
                signature_script: vec![
                    71, 48, 68, 2, 32, 32, 22, 226, 114, 88, 254, 56, 44, 92, 206, 40, 24, 81, 166,
                    48, 33, 140, 13, 78, 133, 100, 167, 176, 172, 185, 0, 83, 117, 163, 160, 195,
                    228, 2, 32, 119, 189, 13, 112, 205, 51, 105, 129, 208, 30, 82, 75, 242, 16,
                    144, 204, 26, 192, 85, 228, 196, 191, 86, 106, 48, 68, 106, 210, 123, 94, 50,
                    234, 1, 33, 2, 8, 254, 253, 29, 198, 111, 232, 89, 209, 102, 27, 0, 146, 11,
                    204, 34, 202, 165, 18, 240, 110, 26, 57, 135, 9, 26, 179, 96, 76, 250, 22, 40,
                ],
                sequence: 4294967293,
            }],
            tx_out_count: CompactSize::U8(2),
            tx_out: vec![
                TxOut {
                    value: 1532965,
                    pk_script_bytes: CompactSize::U8(23),
                    pk_script: vec![
                        169, 20, 113, 53, 86, 175, 25, 212, 187, 94, 28, 199, 65, 211, 169, 87,
                        214, 149, 47, 160, 95, 117, 135,
                    ],
                },
                TxOut {
                    value: 6848866737,
                    pk_script_bytes: CompactSize::U8(23),
                    pk_script: vec![
                        169, 20, 125, 185, 45, 118, 37, 190, 101, 8, 79, 176, 172, 35, 106, 53, 17,
                        103, 6, 150, 63, 13, 135,
                    ],
                },
            ],
            lock_time: 2437014,
        };

        let public_key_script = &prev_tx.tx_out[1].pk_script;

        assert!(evaluate_script(public_key_script.to_vec(), curr_tx, 0,));
    }

    // blockhash: 0000000000000004b84fac97f36ad5455e6521f36c15db1dcda5b61817c8b7b8
    // tx nro 3
    #[test]
    fn test_p2pkh_script2() {
        // txid 1f12db54379a5652918435f28396afb7fed204a23bfb95ce75a826e66eebcc20
        let raw_tx = RawTransaction {
            version: 1,
            tx_in_count: CompactSize::U8(1),
            tx_in: vec![TxIn {
                previous_output: Outpoint {
                    hash: [
                        252, 19, 75, 244, 202, 116, 240, 130, 133, 43, 20, 84, 59, 120, 219, 88,
                        197, 18, 195, 194, 77, 71, 227, 52, 88, 106, 94, 241, 68, 159, 236, 35,
                    ],
                    index: 0,
                },
                script_bytes: CompactSize::U8(106),
                signature_script: vec![
                    71, 48, 68, 2, 32, 16, 114, 141, 101, 111, 81, 134, 112, 25, 141, 89, 61, 144,
                    91, 193, 39, 43, 188, 8, 179, 216, 73, 62, 109, 94, 216, 171, 164, 229, 102, 5,
                    230, 2, 32, 122, 7, 193, 75, 159, 19, 39, 164, 212, 47, 175, 242, 219, 186,
                    222, 61, 116, 88, 135, 4, 230, 2, 145, 176, 153, 168, 111, 90, 9, 176, 63, 19,
                    1, 33, 2, 93, 59, 12, 93, 35, 206, 37, 200, 100, 49, 249, 208, 60, 72, 193, 45,
                    181, 162, 5, 136, 113, 24, 24, 209, 233, 64, 207, 220, 90, 74, 146, 123,
                ],
                sequence: 4294967295,
            }],
            tx_out_count: CompactSize::U8(1),
            tx_out: vec![TxOut {
                value: 274450,
                pk_script_bytes: CompactSize::U8(25),
                pk_script: vec![
                    118, 169, 20, 129, 152, 80, 20, 9, 32, 222, 234, 207, 238, 58, 99, 25, 56, 7,
                    218, 234, 143, 197, 210, 136, 172,
                ],
            }],
            lock_time: 0,
        };

        let pk_hash = [
            11, 139, 32, 119, 74, 146, 223, 9, 212, 72, 207, 66, 73, 35, 72, 27, 52, 87, 236, 54,
        ];

        let public_key_script = PubKeyScript::P2PKH(pk_hash.to_vec()).to_vec();

        assert!(evaluate_script(public_key_script, raw_tx, 0,));
    }
}
