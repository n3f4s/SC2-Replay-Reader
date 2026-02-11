use num_bigint::{ ToBigInt, ToBigUint, Sign, BigInt };
use std::collections::HashMap;

use nom::{
    bytes::complete::{ take, tag, take_while },
    sequence::{ pair },
    combinator::map,
    number::complete::*,
    multi::count,
    branch::alt,
    IResult,
};

use crate::utils::*;
use super::structs::*;

fn fst(d: &[u8]) -> u8 { d[0] }

pub fn parse_vint(data: &[u8], _bit_shift: u64) -> IResult<&[u8], (DataType, u64)> {
    let (data, _) = tag([0x09])(data)?;
    let (mut data, mut byte) = map(take(1_u8), fst)(data)?;
    let is_neg = (byte & 0x01) == 1;
    let mut result = ((byte & 0x7F) >> 1).to_biguint().unwrap();
    let mut bits = 6;
    while (byte & 0x80) != 0 {
        (data, byte) = le_u8(data)?;
        result = ((byte & 0x7F) << bits).to_biguint().unwrap();
        bits += 7;
    };
    Ok((data,
        (DataType::VInt( BigInt::from_biguint(
            if is_neg { Sign::Minus } else { Sign::Plus },
            result
        )), 0))
    )
}

pub fn parse_array(data: &[u8], bit_shift: u64) -> IResult<&[u8], (DataType, u64)> {
    let (data, _) = tag([0x00])(data)?;
    let (mut data, size) = parse_vint(data, bit_shift)?;
    let (size, mut bit_left) = match size {
        (DataType::VInt(i), bit_left) => ((i.to_u64_digits().1)[0], bit_left),
        _ => panic!("Why is it not a VInt ?"),
    };
    let mut res = Vec::new(); // FIXME reserve size
    for _ in 0..size {
        let (d, (s, b)) = parse_struct(data, bit_left)?;
        data = d;
        bit_left = b;
        res.push(Box::new(s));
    }
    Ok((data, (DataType::Array(res), bit_left)))
}

fn _parse_bits(data: &[u8], bit_shift: u64, size: u64) -> IResult<&[u8], (DataType, u64)> {
    let mut size = size;
    let mut res = Vec::new();
    if bit_shift > 0 {
        let (_, val) = le_u8(data)?;
        res.push((val << bit_shift) >> bit_shift);
        size -= bit_shift;
    }
    let mut buffer: u8 = 0;
    let mut data = data;
    while size > 8 {
        (data, buffer) = le_u8(data)?;
        res.push(buffer);
        size -= 8;
    }
    let bits_left = if size == 0 { 0 } else { 8 - size };
    if size > 0 {
        let (_, val) = le_u8(data)?;
        res.push(val >> size);
    }
    Ok((data, (DataType::Blob(res), bits_left)))
}

pub fn parse_bits(data: &[u8], bit_shift: u64) -> IResult<&[u8], (DataType, u64)> {
    let (data, _) = tag([0x01])(data)?;
    let (mut data, size) = parse_vint(data, bit_shift)?;
    let mut size = match size {
        (DataType::VInt(i), _) => (i.to_u64_digits().1)[0],
        _ => panic!("Why is it not a VInt ?"),
    };
    _parse_bits(data, bit_shift, size)
}

pub fn parse_blob(data: &[u8], bit_shift: u64) -> IResult<&[u8], (DataType, u64)> {
    let (data, _) = tag([0x02])(data)?;
    let (data, size) = parse_vint(data, bit_shift)?;
    let size = match size {
        (DataType::VInt(i), _) => (i.to_u64_digits().1)[0],
        _ => panic!("Why is it not a VInt ?"),
    };
    _parse_bits(data, bit_shift, size*8)
}

pub fn parse_choice(data: &[u8], bit_shift: u64) -> IResult<&[u8], (DataType, u64)> {
    let (data, _) = tag([0x03])(data)?;
    parse_struct(data, bit_shift)
}

pub fn parse_optional(data: &[u8], bit_shift: u64) -> IResult<&[u8], (DataType, u64)> {
    let (data, _) = tag([0x04])(data)?;
    let (data, exists) = le_u8(data)?;
    if exists == 0 {
        Ok((data, (DataType::Optional(None), bit_shift)))
    } else {
        parse_struct(data, bit_shift)
    }
}

pub fn parse_assoc_table(data: &[u8], bit_shift: u64) -> IResult<&[u8], (DataType, u64)> {
    let (data, _) = tag([0x05])(data)?;
    let (mut data, size) = parse_vint(data, bit_shift)?;
    let size = match size {
        (DataType::VInt(i), _) => (i.to_u64_digits().1)[0],
        _ => panic!("Why is it not a VInt ?"),
    };
    let mut res = HashMap::new();
    let mut bit_left = 0;
    for _ in 0..size {
        let (d, (k, _)) = parse_vint(data, 0)?;
        let (d, (v, b)) = parse_struct(d, 0)?;
        data = d;
        match k {
            DataType::VInt(k) => res.insert(k, Box::new(v)),
            _ => panic!("Why is the key not a VInt?"),
        };
        bit_left = b;
    }
    Ok((data, (DataType::HashMap(res), bit_left)))
}

pub fn parse_u8(data: &[u8], bit_shift: u64) -> IResult<&[u8], (DataType, u64)> {
    let (data, _) = tag([0x06])(data)?;
    let (data, v) = le_u8(data)?;
    Ok((data, (DataType::UInt8(v), 0)))
}
pub fn parse_u32(data: &[u8], bit_shift: u64) -> IResult<&[u8], (DataType, u64)> {
    let (data, _) = tag([0x06])(data)?;
    let (data, v) = le_u32(data)?;
    Ok((data, (DataType::UInt32(v), 0)))
}
pub fn parse_u64(data: &[u8], bit_shift: u64) -> IResult<&[u8], (DataType, u64)> {
    let (data, _) = tag([0x06])(data)?;
    let (data, v) = le_u64(data)?;
    Ok((data, (DataType::UInt64(v), 0)))
}

fn wrap<F>(f: F) -> Box<dyn Fn(&[u8]) -> IResult<&[u8], (DataType, u64)>>
                    where F: Fn(&[u8], u64) -> IResult<&[u8], (DataType, u64)> + 'static {
    Box::new(move |d| f(d, 0))
}

pub fn parse_struct(data: &[u8], bit_shift: u64) -> IResult<&[u8], (DataType, u64)> {
    let data = if bit_shift > 0 {
        &data[1..]
    } else {
        data
    };
    /* FIXME : 2 options to try :
    - make a function `wrap: (&[u8] -> u64 -> Result) -> (&[u8] -> Result)` to bind the second argument
    - make per function wrapper
     */
    alt((
        wrap(parse_array),
        wrap(parse_blob),
        wrap(parse_bits),
        wrap(parse_choice),
        wrap(parse_optional),
        wrap(parse_struct),
        wrap(parse_assoc_table),
        wrap(parse_u8),
        wrap(parse_u32),
        wrap(parse_u64),
    ))(data)
}
