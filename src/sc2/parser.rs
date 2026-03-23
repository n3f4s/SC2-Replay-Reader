use num_bigint::{ ToBigUint, Sign, BigInt };
use std::collections::HashMap;

use nom::{
    bytes::complete::{ take, tag },
    combinator::map,
    number::complete::*,
    branch::alt,
    IResult,
};

use super::structs::*;

fn fst(d: &[u8]) -> u8 { d[0] }

fn parse_size(data: &[u8]) -> IResult<&[u8], u64> {
    let (data, (size, _)) = parse_vint(data, 0)?;
    match size {
        // XXX We assume that the size of the data won't expect u64_max
        DataType::VInt(i) => {
            if i > BigInt::ZERO {
                Ok((data, i.to_u64_digits().1[0]))
            } else {
                Ok((data, 0))
            }
        },
        _ => panic!("Unexpected size type"),
    }
}

pub fn parse_vint(data: &[u8], _bit_shift: u64) -> IResult<&[u8], (DataType, u64)> {

    let sign_mask = 0x01;
    let mask      = 0x7F;
    let cont_mask = 0x80;


    /*
    First byte: the rightmost bit represent the sign bit
    For all the bytes: the leftmost bit represent the continuation byte
     */

    let (mut data, mut byte) = le_u8(data)?;
    let is_neg = (byte & sign_mask) == 1;
    let mut result = ((byte & mask) >> 1).to_biguint().unwrap();
    let mut bits = 6;
    while (byte & cont_mask) != 0 {
        (data, byte) = le_u8(data)?;
        let byte = (byte & mask).to_biguint().unwrap();
        result |= byte << bits;
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
    let (mut data, size) = parse_size(data)?;
    let mut res = Vec::new(); // FIXME reserve size
    let mut bit_left = 0;
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
    let mut data = data;
    while size >= 8 {
        let (d, buffer) = le_u8(data)?;
        data = d;
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
    let (data, size) = parse_size(data)?;
    _parse_bits(data, bit_shift, size)
}

pub fn parse_blob(data: &[u8], bit_shift: u64) -> IResult<&[u8], (DataType, u64)> {
    let (data, size) = parse_size(data)?;
    _parse_bits(data, bit_shift, size*8)
}

pub fn parse_choice(data: &[u8], bit_shift: u64) -> IResult<&[u8], (DataType, u64)> {
    parse_struct(data, bit_shift)
}

pub fn parse_optional(data: &[u8], bit_shift: u64) -> IResult<&[u8], (DataType, u64)> {
    let (data, exists) = le_u8(data)?;
    if exists == 0 {
        Ok((data, (DataType::Optional(None), bit_shift)))
    } else {
        parse_struct(data, bit_shift)
    }
}

pub fn parse_assoc_table(data: &[u8], bit_shift: u64) -> IResult<&[u8], (DataType, u64)> {
    let (mut data, size) = parse_size(data)?;
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

pub fn parse_u8(data: &[u8], _bit_shift: u64) -> IResult<&[u8], (DataType, u64)> {
    let (data, v) = le_u8(data)?;
    Ok((data, (DataType::UInt8(v), 0)))
}
pub fn parse_u32(data: &[u8], _bit_shift: u64) -> IResult<&[u8], (DataType, u64)> {
    let (data, v) = le_u32(data)?;
    Ok((data, (DataType::UInt32(v), 0)))
}
pub fn parse_u64(data: &[u8], _bit_shift: u64) -> IResult<&[u8], (DataType, u64)> {
    let (data, v) = le_u64(data)?;
    Ok((data, (DataType::UInt64(v), 0)))
}

pub fn parse_struct(data: &[u8], bit_shift: u64) -> IResult<&[u8], (DataType, u64)> {
    let data = if bit_shift > 0 {
        &data[1..]
    } else {
        data
    };
    let (data, typ) = le_u8(data)?;
    match typ {
        0x00 => parse_array(data, 0),
        0x01 => parse_bits(data, 0),
        0x02 => parse_blob(data, 0),
        0x03 => parse_choice(data, 0),
        0x04 => parse_optional(data, 0),
        0x05 => parse_assoc_table(data, 0),
        0x06 => parse_u8(data, 0),
        0x07 => parse_u32(data, 0),
        0x08 => parse_u64(data, 0),
        0x09 => parse_vint(data, 0),
        _ => panic!("Unexpected type tag {}", typ),
    }
}
