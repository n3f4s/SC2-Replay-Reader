use std::str::from_utf8;

use encoding::{ all::ISO_8859_1, all::WINDOWS_1252, DecoderTrap, Encoding };

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

macro_rules! typed_get {
    ($hash:ident, $idx:literal, $rtype:path, $expr:expr) => {
        match &**$hash.get(&$idx.to_bigint().unwrap()).unwrap() {
            $rtype(value) => $expr(value),
            v => panic!("Expected {}, got {:?} for {} at index {}",
                        stringify!($rtype), v, stringify!($hash), $idx),
        }
    };
    ($hash:ident, $idx:literal, $rtype:path) => {
        match &**$hash.get(&$idx.to_bigint().unwrap()).unwrap() {
            $rtype(value) => value,
            v => panic!("Expected {}, got {:?} for {} at index {}",
                        stringify!($rtype), v, stringify!($hash), $idx),
        }
    };
    ($hash:ident, $idx:expr, $rtype:path) => {
        match &**$hash.get(&$idx).unwrap() {
            $rtype(value) => value,
            v => panic!("Expected {}, got {:?} for {} at index {}",
                        stringify!($rtype), v, stringify!($hash), $idx),
        }
    };
    ($hash:ident, $idx:expr, $rtype:path, $expr:expr) => {
        match &**$hash.get(&$idx).unwrap() {
            $rtype(value) => $expr(value),
            v => panic!("Expected {}, got {:?} for {} at index {}",
                        stringify!($rtype), v, stringify!($hash), $idx),
        }
    };
}
pub(crate) use typed_get;

macro_rules! typed_expect {
    ($val:expr, $rtype:path, $expr:expr) => {
        match $val {
            $rtype(value) => $expr(value),
            v => panic!("Expected {} : {:?}", stringify!($rytpe), v),
        }
    };
    ($val:expr, $rtype:path) => {
        match $val {
            $rtype(value) => value,
            v => panic!("Expected {} : {:?}", stringify!($rytpe), v),
        }
    };
}
pub(crate) use typed_expect;

macro_rules! _parse_hm_mem {
    ($mem:ident |> $idx:literal => $rtype:path => $expr:expr) => {
        $mem: typed_get!($hashmap, $idx, $rtype,$expr),
    }
}

macro_rules! parse_hashmap {
    ($hashmap:ident |> $cls:path  { $($mem:ident |> $idx:literal => $rtype:path => $expr:expr, )+ } ) => {{
        $cls {
            $($mem: typed_get!($hashmap, $idx, $rtype,$expr),)+
        }
   }}
}
pub(crate) use parse_hashmap;

macro_rules! _parse_single_mem {
    (uint32 $prev:ident $shift:ident $mem:ident $cls:path, $data:ident ) => {
        let ($data, ($mem, _)) = parse_u32($data, 0)?;
    };
    ({ aligned_bits $size:literal } $prev:ident $shift:ident $mem:ident $cls:path, $data:ident ) => {
        let ($data, ($mem, _, _)) = _parse_bits($data, 0, $size, 0)?;
    };
    ({ aligned_string $size:literal } $prev:ident $shift:ident $mem:ident $cls:path, $data:ident ) => {
        let ($data, ($mem, _, _)) = _parse_bits($data, 0, $size, 0)?;
        let $mem = typed_expect!($mem, DataType::Blob, |v: Vec<u8>| decode_utf8(&v).unwrap());
        let $mem = DataType::Str($mem);
    };
}
pub(crate) use _parse_single_mem;

macro_rules! parse_event {
    ($name:literal : $data:ident { $($mem:ident -> $typ:tt of $cls:path,)+ }) => {{
        let mut prev_bits = 0;
        let mut shift_bits = 0;
        $(_parse_single_mem!($typ prev_bits shift_bits $mem $cls, $data);)+
        Ok(($data,
            Some(($name,
                  HashMap::from([
                      $((stringify!($mem), $mem),)+
                  ])
            ))
        ))
    }};
}
pub(crate) use parse_event;

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

pub fn decode_utf8(data: &[u8]) -> Result<String, ()> { // FIXME improve
    match str::from_utf8(&data) {
        Ok(s) => Ok(s.to_string()),
        Err(_) => Err(()),
    }
}

pub fn decode_iso(data: &[u8]) -> Result<String, ()> { // FIXME improve
    match ISO_8859_1.decode(data, DecoderTrap::Strict) {
        Ok(s) => Ok(s.to_string()),
        Err(_) => match from_utf8(data) {
            Ok(s) => Ok(s.to_string()),
            Err(_) => Err(())
        },
    }
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
            let mask = mask << (8*i);
            let v = (d & mask) >> (8*i);
            res.push(v.try_into().unwrap());
        }
    }
    res
}
