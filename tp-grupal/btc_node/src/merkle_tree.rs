use bitcoin_hashes::{sha256d, Hash};

pub fn merkle_tree_root(txns: Vec<[u8; 32]>) -> [u8; 32] {
    if txns.len() == 1 {
        txns[0]
    } else {
        _merkle_tree_root(txns)
    }
}

fn _merkle_tree_root(txns: Vec<[u8; 32]>) -> [u8; 32] {
    let mut resultados: Vec<[u8; 32]> = Vec::new();
    for slice in txns.chunks(2) {
        let one = slice.first();
        let two = slice.get(1);

        let c = match one {
            Some(tx1) => match two {
                Some(tx2) => [*tx1, *tx2].concat(),
                None => [*tx1, *tx1].concat(),
            },
            None => return [0u8; 32],
        };

        resultados.push(sha256d::Hash::hash(&c).to_byte_array());
    }

    if resultados.len() == 1 {
        resultados[0]
    } else {
        _merkle_tree_root(resultados)
    }
}
