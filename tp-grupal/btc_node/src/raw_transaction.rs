use crate::{
    blockchain::utxo_set::Output,
    constants::{SIGHASH_ALL, TX_VERSION},
    message::compact_size::CompactSize,
    protocol_error::ProtocolError,
    utils::wif_to_private_key,
};

use bitcoin_hashes::{sha256d, Hash};
use secp256k1::{ecdsa, Message, PublicKey, Secp256k1, SecretKey};

use std::{io::Read, num::ParseIntError};

#[derive(Debug, Clone)]
pub struct RawTransaction {
    pub version: i32,
    pub tx_in_count: CompactSize,
    pub tx_in: Vec<TxIn>,
    pub tx_out_count: CompactSize,
    pub tx_out: Vec<TxOut>,
    pub lock_time: u32,
}

impl RawTransaction {
    pub fn new(txin: Vec<TxIn>, txout: Vec<TxOut>) -> RawTransaction {
        RawTransaction {
            version: 1,
            tx_in_count: CompactSize::new_from_usize(txin.len()),
            tx_in: txin,
            tx_out_count: CompactSize::new_from_usize(txout.len()),
            tx_out: txout,
            lock_time: 0,
        }
    }

    pub fn read_from(stream: &mut dyn Read) -> Result<RawTransaction, ProtocolError> {
        let mut version: [u8; 4] = [0; 4];
        stream.read_exact(&mut version)?;

        let tx_in_count = CompactSize::read_from(stream)?;
        let mut tx_in = Vec::new();
        for _i in 0..tx_in_count.into_inner() {
            tx_in.push(TxIn::read_from(stream)?);
        }

        let tx_out_count = CompactSize::read_from(stream)?;
        let mut tx_out = Vec::new();
        for _i in 0..tx_out_count.into_inner() {
            tx_out.push(TxOut::read_from(stream)?);
        }

        let mut lock_time: [u8; 4] = [0; 4];
        stream.read_exact(&mut lock_time)?;

        Ok(RawTransaction {
            version: (i32::from_le_bytes(version)),
            tx_in_count,
            tx_in,
            tx_out_count,
            tx_out,
            lock_time: (u32::from_le_bytes(lock_time)),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes.extend_from_slice(&self.tx_in_count.to_le_bytes());

        for i in 0..self.tx_in_count.into_inner() {
            bytes.extend_from_slice(&self.tx_in[i].to_bytes());
        }

        bytes.extend_from_slice(&self.tx_out_count.to_le_bytes());
        for i in 0..self.tx_out_count.into_inner() {
            bytes.extend_from_slice(&self.tx_out[i].to_bytes());
        }

        bytes.extend_from_slice(&self.lock_time.to_le_bytes());

        bytes
    }

    pub fn get_tx_id(&self) -> [u8; 32] {
        sha256d::Hash::hash(&self.to_bytes()[..]).to_byte_array()
    }

    pub fn get_tx_value(&self) -> i64 {
        let mut value = 0;

        for tx in &self.tx_out {
            value += tx.value;
        }
        value
    }

    pub fn get_utxos(&self) -> Vec<(([u8; 32], u32), TxOut)> {
        let mut utxos = Vec::new();
        let id = self.get_tx_id();
        let mut index_output = 0;
        for tx in &self.tx_out {
            utxos.push(((id, index_output), tx.clone()));
            index_output += 1;
        }
        utxos
    }

    pub fn get_tx_inputs(&self) -> Vec<([u8; 32], u32)> {
        let mut inputs = Vec::new();
        for tx in &self.tx_in {
            inputs.push((tx.get_outpoint_hash(), tx.get_outpoint_index()));
        }
        inputs
    }

    pub fn serialize(&self, input: usize, pubkey_script: Vec<u8>) -> Vec<u8> {
        let mut s: Vec<u8> = vec![];

        s.extend_from_slice(&self.version.to_le_bytes());
        s.extend_from_slice(&self.tx_in_count.to_le_bytes());
        for (i, txin) in self.tx_in.iter().enumerate() {
            s.extend_from_slice(&txin.previous_output.to_bytes());
            if i == input {
                let len_pubkey = CompactSize::new_from_usize(pubkey_script.len());
                s.extend_from_slice(&len_pubkey.to_le_bytes());
                s.extend_from_slice(&pubkey_script);
            } else {
                s.push(0);
            }
            s.extend_from_slice(&txin.sequence.to_le_bytes());
        }
        s.extend_from_slice(&self.tx_out_count.to_le_bytes());
        for txout in &self.tx_out {
            s.extend_from_slice(&txout.to_bytes());
        }
        s.extend_from_slice(&self.lock_time.to_le_bytes());
        s.extend_from_slice(&(1u32).to_le_bytes());

        s
    }

    pub fn create_transaction(
        out_to_spend: Vec<([u8; 32], Output)>,
        tx_out: Vec<TxOut>,
        wif_private_key: &str,
    ) -> RawTransaction {
        let mut tx_in = vec![];
        for (hash, out) in out_to_spend.iter() {
            let previous_output = Outpoint {
                hash: *hash,
                index: out.index,
            };
            tx_in.push(TxIn::new(previous_output, vec![]));
        }

        let tx_in_count = CompactSize::new_from_usize(tx_in.len());
        let tx_out_count = CompactSize::new_from_usize(tx_out.len());

        let mut tx = RawTransaction {
            version: TX_VERSION,
            tx_in_count: tx_in_count.clone(),
            tx_in,
            tx_out_count,
            tx_out,
            lock_time: 0,
        };

        let private_key = wif_to_private_key(wif_private_key);

        let secp = Secp256k1::signing_only();
        let secret_key = SecretKey::from_slice(&private_key).unwrap();
        let public_key = PublicKey::from_secret_key(&secp, &secret_key).serialize();
        let public_key_len = &CompactSize::new_from_usize(public_key.len()).to_le_bytes()[..];

        for i in 0..tx_in_count.into_inner() {
            let signature = tx.serialize(i, out_to_spend[i].1.pkscript.to_vec());
            let signature_hash = sha256d::Hash::hash(&signature).to_byte_array();

            let message = Message::from_slice(&signature_hash).unwrap();
            let _sig = secp.sign_ecdsa(&message, &secret_key);
            let sig = &ecdsa::Signature::serialize_der(&_sig).to_vec()[..];
            let len_sig = &CompactSize::new_from_usize(sig.len() + 1).to_le_bytes()[..];

            let signature_script =
                [len_sig, sig, &[SIGHASH_ALL], public_key_len, &public_key].concat();

            tx.tx_in[i].script_bytes = CompactSize::new_from_usize(signature_script.len());
            tx.tx_in[i].signature_script = signature_script;
        }

        tx
    }
}

#[derive(Debug, Clone)]
pub struct Outpoint {
    pub hash: [u8; 32],
    pub index: u32,
}

impl Outpoint {
    pub fn new(hash: [u8; 32], index: u32) -> Outpoint {
        Outpoint { hash, index }
    }

    pub fn read_from(stream: &mut dyn Read) -> Result<Outpoint, ProtocolError> {
        let mut hash: [u8; 32] = [0; 32];
        stream.read_exact(&mut hash)?;

        let mut index: [u8; 4] = [0; 4];
        stream.read_exact(&mut index)?;

        Ok(Outpoint {
            hash,
            index: u32::from_le_bytes(index),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.hash[..]);
        bytes.extend_from_slice(&self.index.to_le_bytes());

        bytes
    }
}

#[derive(Debug, Clone)]
pub struct TxIn {
    pub previous_output: Outpoint,
    pub script_bytes: CompactSize,
    pub signature_script: Vec<u8>,
    pub sequence: u32,
}

impl TxIn {
    pub fn new(previous_output: Outpoint, signature_script: Vec<u8>) -> TxIn {
        TxIn {
            previous_output,
            script_bytes: CompactSize::new_from_usize(signature_script.len()),
            signature_script,
            sequence: 0xffffffff,
        }
    }

    pub fn read_from(stream: &mut dyn Read) -> Result<TxIn, ProtocolError> {
        let previous_output = Outpoint::read_from(stream)?;
        let script_bytes = CompactSize::read_from(stream)?;
        let mut signature_script: Vec<u8> = Vec::new();
        let mut byte: [u8; 1] = [0];
        for _i in 0..script_bytes.into_inner() {
            stream.read_exact(&mut byte)?;
            signature_script.push(byte[0]);
        }

        let mut sequence: [u8; 4] = [0; 4];
        stream.read_exact(&mut sequence)?;

        Ok(TxIn {
            previous_output,
            script_bytes,
            signature_script,
            sequence: u32::from_le_bytes(sequence),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.previous_output.to_bytes());
        bytes.extend_from_slice(&self.script_bytes.to_le_bytes());
        bytes.extend_from_slice(&self.signature_script[..]);
        bytes.extend_from_slice(&self.sequence.to_le_bytes());

        bytes
    }

    fn get_outpoint_index(&self) -> u32 {
        self.previous_output.index
    }

    fn get_outpoint_hash(&self) -> [u8; 32] {
        self.previous_output.hash
    }
}

#[derive(Debug, Clone)]
pub struct TxOut {
    pub value: i64,
    pub pk_script_bytes: CompactSize,
    pub pk_script: Vec<u8>,
}

impl TxOut {
    pub fn new(value: i64, pk_script: Vec<u8>) -> TxOut {
        TxOut {
            value,
            pk_script_bytes: CompactSize::new_from_usize(pk_script.len()),
            pk_script,
        }
    }

    pub fn read_from(stream: &mut dyn Read) -> Result<TxOut, ProtocolError> {
        let mut value: [u8; 8] = [0; 8];
        stream.read_exact(&mut value)?;

        let pk_script_bytes = CompactSize::read_from(stream)?;
        let mut pk_script: Vec<u8> = Vec::new();
        let mut byte: [u8; 1] = [0];
        for _i in 0..pk_script_bytes.into_inner() {
            stream.read_exact(&mut byte)?;
            pk_script.push(byte[0]);
        }

        Ok(TxOut {
            value: i64::from_le_bytes(value),
            pk_script_bytes,
            pk_script,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.value.to_le_bytes());
        bytes.extend_from_slice(&self.pk_script_bytes.to_le_bytes());
        bytes.extend_from_slice(&self.pk_script[..]);

        bytes
    }

    pub fn get_pk(&self) -> Vec<u8> {
        self.pk_script.clone()
    }
}

pub fn unhexlify(hex: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
        .collect()
}

#[test]
fn test_tx_id() {
    let hash: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ];
    let outpoint = Outpoint { hash, index: 0 };
    let txin = TxIn {
        previous_output: outpoint,
        script_bytes: CompactSize::U8(41),
        signature_script: unhexlify(
            "034a771b0468ef265f2f53424943727970746f2e636f6d20506f6f6c2f012435c22d21000000000000",
        )
        .unwrap(),
        sequence: 4294967295,
    };
    let txout1 = TxOut {
        value: 19531532,
        pk_script_bytes: CompactSize::U8(22),
        pk_script: unhexlify("00140e289c4c6030afc3fef48b94bf4ebdf5a12e67a9").unwrap(),
    };
    let txout2 = TxOut {
        value: 0,
        pk_script_bytes: CompactSize::U8(38),
        pk_script: unhexlify(
            "6a24aa21a9edfc844ef4df361af723e511ab354d5c7bff793395268aceb2e1a24ab55ebc2fcb",
        )
        .unwrap(),
    };

    let mut txins = Vec::new();
    txins.push(txin);

    let mut txouts = Vec::new();
    txouts.push(txout1);
    txouts.push(txout2);

    let tx = RawTransaction {
        version: 2,
        tx_in_count: CompactSize::U8(1),
        tx_in: txins,
        tx_out_count: CompactSize::U8(2),
        tx_out: txouts,
        lock_time: 0,
    };

    println!("{:?}", tx.get_tx_id());

    let outpoint2 = Outpoint {
        hash: unhexlify("b3c8723018e3871ab0fee00c8209e127544b190949cca121c44e9d1ed64470f3")
            .unwrap()
            .try_into()
            .unwrap(),
        index: 0,
    };
    let txin2 = TxIn {
        previous_output: outpoint2,
        script_bytes: CompactSize::U8(0),
        signature_script: unhexlify("").unwrap(),
        sequence: 4294967295,
    };
    let txout21 = TxOut{value:0, pk_script_bytes: CompactSize::U8(83), pk_script:unhexlify("6a4c50000b42e40002a11a54b3a6421e62384cf2ea0963791070430057d0e91593a6dca0b24c1bdcb1f3cfbf94328aac2cd249ccc5fa355f26ef48050282f370ccf90a27de9305ad5bc9212a4ce706aebc7497").unwrap()};
    let txout22 = TxOut {
        value: 554158,
        pk_script_bytes: CompactSize::U8(22),
        pk_script: unhexlify("0014dae041024ba702765d968878162bdf0afee92826").unwrap(),
    };

    let mut txins2 = Vec::new();
    txins2.push(txin2);

    let mut txouts2 = Vec::new();
    txouts2.push(txout21);
    txouts2.push(txout22);

    let tx = RawTransaction {
        version: 1,
        tx_in_count: CompactSize::U8(1),
        tx_in: txins2,
        tx_out_count: CompactSize::U8(2),
        tx_out: txouts2,
        lock_time: 0,
    };

    println!("{:?}", tx.get_tx_id());
}
