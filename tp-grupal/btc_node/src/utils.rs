use crate::{constants::P2PKH_BYTE, protocol_error::ProtocolError};
use bitcoin_hashes::{ripemd160, sha256, sha256d, Hash};
use secp256k1::{PublicKey, Secp256k1, SecretKey};

pub fn hash160(bytes: &[u8]) -> [u8; 20] {
    ripemd160::Hash::hash(&sha256::Hash::hash(bytes).to_byte_array()).to_byte_array()
}

pub fn bitcoin_address_to_pkhash(addrs: &str) -> Result<Vec<u8>, ProtocolError> {
    let address_decoded = bs58::decode(addrs)
        .into_vec()
        .map_err(|_| ProtocolError::Error("Error decoding the base58 address".to_string()))?;

    let l = address_decoded.len();
    let checksum = &address_decoded[(l - 4)..l];
    let check = &sha256d::Hash::hash(&address_decoded[..(l - 4)]).to_byte_array()[0..4];

    if check != checksum {
        return Err(ProtocolError::Error(
            "Address has invalid checksum".to_string(),
        ));
    }

    Ok(address_decoded[1..21].to_vec())
}

pub fn wif_to_private_key(wif: &str) -> Vec<u8> {
    let wif_decoded = &bs58::decode(wif).into_vec().unwrap()[..];
    let private_key = &wif_decoded[1..(wif_decoded.len() - 5)];
    private_key.to_vec()
}

pub fn wif_to_bitcoin_address(wif: &str) -> String {
    let pkhash = wif_to_pkhash(wif).unwrap();
    let mut addr = [&[P2PKH_BYTE], &pkhash[..]].concat();
    let checksum = &sha256d::Hash::hash(&addr).to_byte_array()[0..4];

    addr.extend_from_slice(checksum);

    bs58::encode(addr).into_string()
}

pub fn wif_to_pkhash(wif: &str) -> Result<[u8; 20], ProtocolError> {
    let private_key = crate::utils::wif_to_private_key(wif);
    let secp = Secp256k1::signing_only();
    let secret_key = SecretKey::from_slice(&private_key)
        .map_err(|_| ProtocolError::Error("Converting the wif to a private key".to_string()))?;

    let public_key = PublicKey::from_secret_key(&secp, &secret_key).serialize();

    Ok(hash160(&public_key))
}

pub fn bytes_to_hex_string(bytes: &[u8]) -> String {
    let hex_chars: Vec<String> = bytes.iter().map(|b| format!("{:02X}", b)).collect();
    hex_chars.join("").to_lowercase()
}

pub fn decode_hex(s: &str) -> [u8; 32] {
    let mut hash: [u8; 32] = [0; 32];
    for i in 0..64 {
        if i % 2 == 1 {
            continue;
        }
        hash[31 - i / 2] = u8::from_str_radix(&s[i..(i + 2)], 16).unwrap();
    }
    hash
}
