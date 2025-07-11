use std::str::from_utf8;

use encoding::{ all::ISO_8859_1, all::WINDOWS_1252, DecoderTrap, Encoding };
use num_bigint::{ BigUint, ToBigUint };

macro_rules! compose {
    ( $last:expr ) => { $last };
    ( $head:expr, $($tail:expr), +) => {
        compose_two($head, compose!($($tail),+))
    };
}
pub(crate) use compose;

macro_rules! pipe {
    ($init:tt $(=>$fn:ident)+) => {{
        let r = $init;
        $( let r = $fn(r); )*
            r
    }}
}
pub(crate) use pipe;

pub fn compose_two<A, B, C, G, F>(f: F, g: G) -> impl Fn(A) -> C
where
    F: Fn(A) -> B,
    G: Fn(B) -> C,
{
    move |x| g(f(x))
}

pub fn snd<'a>((_, res): (&[u8], &'a [u8])) -> &'a [u8] {
    res
}

pub fn decode(data: &[u8]) -> Result<String, ()> { // FIXME improve
    match WINDOWS_1252.decode(data, DecoderTrap::Strict) {
        Ok(s) => Ok(s.to_string()),
        Err(_) => match from_utf8(data) {
            Ok(s) => Ok(s.to_string()),
            Err(_) => Err(())
        },
    }
}

pub fn to_u64(vec: &[u8]) -> u64  {
    let mut i = 0;
    let mut res = 0;
    for v in vec.iter().rev() {
        res += (*v as u64) << i;
        i += 8
    }
    res
}

pub fn jump_to_offset(offset: usize, max_size: usize, current_size: usize, data: &[u8]) -> &[u8] {
    let current_pos = max_size - current_size;
    let corrected_offset = offset - current_pos;
    &data[corrected_offset..]
}

pub fn pack_u8_vec(data: &[u8]) -> Vec<u64> {
    // FIXME: reserve size
    let mut res = Vec::new();
    let mut buffer = [0; 4];
    let mut i = 0;
    for d in data {
        buffer[i] = *d;
        if i == 3 {
            i = 0;
            res.push(u32::from_le_bytes(buffer) as u64);
            buffer = [0; 4];
        } else {
            i += 1;
        }
    }
    res
}

pub fn unpack_u64_vec(data: &[u64]) -> Vec<u8> {
    // FIXME endianness with non LE machines ?
    // FIXME: reserve size
    let mut res = Vec::new();
    let mask: u64 = (1 << 8) - 1;
    for d in data {
        for i in 0..4 {
            let mask = mask << 8*i;
            let v = (d & mask) >> 8*i;
            res.push(v.try_into().unwrap());
        }
    }
    res
}
