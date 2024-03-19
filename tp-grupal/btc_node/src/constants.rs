pub const PATH_CONFIG: &str = "config/node.conf";

// TESTNET header start string (magic string)
pub const START_STRING: [u8; 4] = [11, 17, 9, 7];

//Gensis block
//Contains the hash value of the bitcoin test network:
pub const GENESIS_BLOCK_HASH_VALUE: &str =
    "000000000933ea01ad0ee984209779baaec3ced90fa3f408719526f8d77f4943";
pub const GENESIS_BLOCK_MERKLE_ROOT_HASH_VALUE: &str =
    "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b";

pub const BLOCK_DOWNLOADING_START_TIMESTAMP: u32 = 1680318000; // 1/4/2023

pub const P2PKH_BYTE: u8 = 0x6f;
pub const P2SH_BYTE: u8 = 0xc4;

pub const SIGHASH_ALL: u8 = 1u8;
pub const TX_VERSION: i32 = 1;
