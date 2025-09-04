
use super::structs::*;

pub struct CryptTable {
    dw_crypt_table: [u64; 0x500],
}

impl CryptTable {
    pub fn new() -> CryptTable {

        let mut table =  [0u64; 0x500];

        let mut seed = 0x00100001;

        for index1 in 0..0x100 {
            let mut index2 = index1;
            for _ in 0..5 {
                seed  = (seed * 125 + 3) % 0x2AAAAB;
                let temp1 = (seed & 0xFFFF) << 0x10;

                seed  = (seed * 125 + 3) % 0x2AAAAB;
                let temp2 = seed & 0xFFFF;

                table[index2] = temp1 | temp2;

                index2 += 0x100
            }
        }

        CryptTable {
            dw_crypt_table: table,
        }
    }
    // FIXME not return a vec?
    pub fn decrypt(&self, buffer: &[u64], key: u64) -> Vec<u64> {

        let mut res = vec![0; buffer.len()];

        let mut seed1: u128 = key as u128;
        let mut seed2: u128 = 0xEEEEEEEE;
        let mask: u128 = 0xFFFFFFFF; // (1 << 64) - 1;

        for (i, d) in buffer.iter().enumerate() {
            seed2 += self.dw_crypt_table[0x400usize + ((seed1 as usize) & 0xFFusize)] as u128;
            seed2 &= mask;

            let mut val = *d as u128;
            val ^= seed1 + seed2;
            val &= mask;

            seed1 = ((!seed1 << 0x15) + 0x11111111) | (seed1 >> 0x0B);
            seed1 &= mask;

            seed2 = val + seed2 + (seed2 << 5) + 3;
            seed2 &= mask;

            res[i] = val as u64
        }

        // let mut seed: usize = 0xEEEEEEEE;
        // let mut key = key as usize;


        // for ( i, d ) in buffer.iter().enumerate() {
        //     seed += self.dw_crypt_table[0x400usize + (key & 0xFFusize)] as usize;
        //     let ch = ( *d as usize ) ^ (key + seed);
        //     key = ((!key << 0x15) + 0x11111111) | (key >> 0x0B);
        //     seed = ch + seed + (seed << 5) + 3;
        //     res[i] = ch as u64
        // }
        res
    }

    pub fn hash(&self, string: String, hashtype: HashType) -> u64 {
        let mut seed1: u64 = 0x7FED7FED;
        let mut seed2: u64 = 0xEEEEEEEE;
        let hashtype = hashtype as u64;

        for c in string.to_uppercase().chars() {
            let c: u64 = c.into();
            let value = self.dw_crypt_table[((hashtype << 8) + c) as usize];
            seed1 = (value ^ seed1.wrapping_add(seed2)) & 0xFFFFFFFF ;
            seed2 = c.wrapping_add(seed1).wrapping_add(seed2).wrapping_add(seed2 << 5).wrapping_add(3) & 0xFFFFFFFF;
        }
        seed1
    }
}

