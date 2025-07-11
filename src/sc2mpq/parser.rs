
use std::{ env, fs, convert::TryInto };
use std::iter::{Map, zip};
use std::collections::HashMap;
use std::slice::Iter;

use super::structs::*;
use crate::utils::*;

use nom::{
    bytes::complete::{ take, tag, take_while },
    sequence::{ pair },
    combinator::map,
    multi::count,
    branch::alt,
    IResult,
};


pub fn parse_kv(data: &[u8]) -> IResult<&[u8], DataType> {
    let (data, size) = map(pair(tag([0x05]), take(1 as u8)), compose!(snd, |v: &[u8]| v[0] >> 1))(data)?;
    fn to_map(a: Vec<(&[u8], DataType)>) -> DataType {
        let mut res = HashMap::new();
        for (k, v) in a {
            res.insert(k[0] >> 1, Box::new(v));
        }
        DataType::KeyValueObject(res)
    }
    map(count(
        pair(take(1usize), parse_serialized_data),
        size as usize
    ), to_map)(data)
}

pub fn parse_array(data: &[u8]) -> IResult<&[u8], DataType> {
    let (data, size) = map( pair(
        tag([0x04, 0x10, 0x00]),
        take(1usize)
    ), compose!(snd, |e: &[u8]| e[0] >> 1) )(data)?;
    fn map_box(d: Vec<DataType>) -> Vec<Box<DataType>> {
        let mut v = Vec::new();
        for e in d { v.push(Box::new(e)) }
        v
    }
    fn array(e: Vec<Box<DataType>>) -> DataType { DataType::array_object(e) }
    let res = map(
        count(parse_serialized_data, size as usize),
        |d| pipe!( d => map_box => array ))(data);
    res
}

pub fn parse_vlf(data: &[u8]) -> IResult<&[u8], DataType> {
    let mask = 1 << 7;
    let cont = |i: u8| (i & mask) != 0;
    fn concat((e, l): (&[u8], &[u8])) -> Vec<u8> {
        let mut e = Vec::from(e);
        e.push(l[0]);
        e
    }
    let to_vlfi = |(_, v): (&[u8], Vec<u8>)| DataType::vlf_int(v.iter().map(|e| e & !mask).collect());
    let res = map(pair(
        tag([0x09]),
        map(pair(
            take_while(cont),
            take(1u8)
        ), concat)
    ), to_vlfi)(data);
    res
}

pub fn parse_byte_string(data: &[u8]) -> IResult<&[u8], DataType> {
    let (data, size) = map(pair(tag([0x02]), take(1usize)), compose!(snd, |v: &[u8]| v[0]))(data)?;
    let size = size >> 1;
    let (data, content) = take(size)(data)?;
    // let (data, _) = tag([0x02])(data)?; // There seems to have a closing 0x02 at the end of a string
    Ok((data, DataType::ByteString(decode(content).unwrap())))
}

pub fn parse_single_byte_int(data: &[u8]) -> IResult<&[u8], DataType> {
    map(pair(tag([0x06]), take(1usize)), compose!(snd, |v: &[u8]| v[0], DataType::singlebyte_int))(data)
}

pub fn parse_four_bytes_int(data: &[u8]) -> IResult<&[u8], DataType> {
    map(pair(tag([0x07]), take(4usize)), compose!(snd, DataType::fourbytes_int))(data)
}

pub fn parse_serialized_data(data: &[u8]) -> IResult<&[u8], DataType> {
    alt((
        parse_single_byte_int,
        parse_four_bytes_int,
        parse_byte_string,
        parse_array,
        parse_vlf,
        parse_kv,
    ))(data)
}
