use num_bigint::{ ToBigUint, ToBigInt, Sign, BigInt };
use std::collections::HashMap;

use nom::{
    number::complete::*,
    IResult,
};

use super::structs::*;
use super::events::*;
use crate::utils::*;

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

pub fn parse_array(data: &[u8], _bit_shift: u64) -> IResult<&[u8], (DataType, u64)> {
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

/// Read unaligned bytes. If the byte doesn't end up on an align byte and there are
/// left over bits, the bits will be return along with the number of bits left.
/// bit_shift is the number of bit remaining from the last call to _parse_bits
/// prev_bits contains the bit_shift left over bits from the last call to _parse_bits
pub fn _parse_bits(data: &[u8], bit_shift: u64, size: u64, prev_bits: u8) -> IResult<&[u8], (DataType, u64, u8)> {
    let mut size = size;
    let mut res = Vec::new();
    if bit_shift > 0 {
        size -= bit_shift;
        res.push(prev_bits);
    }
    let mut data = data;
    while size >= 8 {
        let (d, buffer) = le_u8(data)?;
        data = d;
        res.push(buffer);
        size -= 8;
    }
    let bits_left = if size == 0 { 0 } else { 8 - size };
    let next_bits = if size > 0 {
        let (_, val) = le_u8(data)?;
        res.push(val >> size);
        ( val << bits_left ) >> bits_left
    } else { 0 };
    Ok((data, (DataType::Blob(res), bits_left, next_bits)))
}

pub fn parse_bits(data: &[u8], bit_shift: u64) -> IResult<&[u8], (DataType, u64)> {
    let (data, size) = parse_size(data)?;
    let (data, (res, bit_shift, _)) = _parse_bits(data, bit_shift, size, 0)?;
    Ok((data, (res, bit_shift)))
}

pub fn parse_blob(data: &[u8], bit_shift: u64) -> IResult<&[u8], (DataType, u64)> {
    let (data, size) = parse_size(data)?;
    let (data, (res, bit_shift, _)) = _parse_bits(data, bit_shift, size*8, 0)?;
    Ok((data, (res, bit_shift)))
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

pub fn parse_assoc_table(data: &[u8], _bit_shift: u64) -> IResult<&[u8], (DataType, u64)> {
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

pub fn parse_frames(data: &[u8]) -> IResult<&[u8], u64> {
    let (data, byte) = le_u8(data)?;
    let time = (byte >> 2) as u64;
    let additional_bytes = byte & 0x03;
    let (data, time) = match additional_bytes {
        0 => (data, time),
        1 => {
            let (data, next) = le_u8(data)?;
            (data, (time << 8) | (next as u64))
        },
        2 => {
            let (data, next) = le_u16(data)?;
            (data, (time << 16) | (next as u64))
        },
        3 => {
            let (data, next) = le_u16(data)?;
            let (data, last) = le_u8(data)?;
            (data, (time << 24) | ((next as u64) << 8) | (last as u64))
        },
        _ => panic!("Unexpected value for additional_bytes : {}", additional_bytes),
    };
    Ok((data, time))
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

fn parse_event(data: &[u8], fstamp: u64, build: u64) -> IResult<&[u8], (Event, u64)> {
    let (data, next_fstamp) = parse_frames(data)?;
    let fstamp = fstamp + next_fstamp;
    let (data, (pid, bits_left, next_bits)) = _parse_bits(data, 0, 5, 0)?;
    let pid = typed_expect!(pid,
                            DataType::Blob,
                            |v: Vec<u8>| {
                                let i = BigInt::from_bytes_le(Sign::Plus, &v);
                                if i > BigInt::ZERO {
                                    i.to_u64_digits().1[0]
                                } else {
                                    0
                                }
                            });
    let (data, (event_type, bits_left, next_bits)) = _parse_bits(data, bits_left, 7, next_bits)?;
    let event_type = typed_expect!(event_type,
                                   DataType::Blob,
                                   |v: Vec<u8>| {
                                       let i = BigInt::from_bytes_le(Sign::Plus, &v);
                                       if i > BigInt::ZERO {
                                           i.to_u64_digits().1[0]
                                       } else {
                                           0
                                       }
                                   }
    );
    let (data, event) = event_dispatcher(build, event_type, data)?;
    let event = match event {
        Some((name, data)) => Event {
            event_id: event_type,
            player: None,
            pid: pid,
            frame: fstamp,
            second: fstamp >> 4,
            is_local: pid != 16,
            name: name.to_string(),
        },
        None => panic!("Unknown event {}", event_type),
    };
    Ok((data, (event, fstamp)))
}

pub fn parse_events(data: &[u8], build: u64) -> IResult<&[u8], Vec<Event>> {
    // FIXME test if the replay is in debug mode ?
    let mut data = data;
    let mut res = Vec::new();
    let mut fstamp = 0;
    while !data.is_empty() {
        let (d, (e, f)) = parse_event(data, fstamp, build)?;
        data = d;
        println!("{:#?}", e);
        res.push(e);
        fstamp = f;
    }
    Ok((data, res))
}

fn decode_blob(v: &[u8]) -> String { decode_utf8(v).unwrap() }

fn id<T: Clone>(i: &T) -> T { i.clone() }

fn parse_thumbnail(thumbnail: &HashMap<BigInt, Box<DataType>>) -> String {
    typed_get!(thumbnail, 0, DataType::Blob, decode_blob)
}

fn parse_cache_handle(cache_handle: &Vec<Box<DataType>>) -> Vec<String> {
    cache_handle.iter().map(
        |v| {
            let bytes = typed_expect!(&**v, DataType::Blob);
            let mut server = decode_utf8(&bytes[4..8]).unwrap().trim_matches(|c| c == '\x00' || c == ' ').to_lowercase();
            let mut res: Vec<u8> = Vec::new();
            for e in &bytes[8..] {
                let s = format!("{:02x}", e);
                res.push(s.bytes().nth(0).unwrap());
                res.push(s.bytes().nth(1).or(Some(0)).unwrap());
            }
            let hash = decode_utf8(&res).unwrap();
            let type_ = decode_utf8(&bytes[0..4]).unwrap();
            let mut scheme = "https";
            let mut domain = "classic.blizzard.com";

            if server == "sea" {
                server = "us".to_string();
            } else if server == "cn" {
                scheme = "http";
                domain = "battlenet.com.cn";
            };

            format!("{}://{}-s2-depot.{}/{}.{}", scheme, server, domain, hash, type_)

        }
    ).collect::<Vec<String>>()
}

fn parse_players(players: &Vec<Box<DataType>>) -> Vec<Player> {
    fn parse_player(player: &HashMap<BigInt, Box<DataType>>) -> Player {
        parse_hashmap! {
            player |> Player {
                name |> 0 => DataType::Blob => decode_blob,
                bnet |> 1 => DataType::HashMap => |hm: &HashMap<BigInt, Box<DataType>>|
                parse_hashmap! { hm |> BNet {
                    region     |> 0 => DataType::VInt => id,
                    program_id |> 1 => DataType::UInt32 => id,
                    subregion  |> 2 => DataType::VInt => id,
                    uid        |> 4 => DataType::VInt => id,

                } },
                race  |> 2 => DataType::Blob => decode_blob,
                color |> 3 => DataType::HashMap => |color_hashmap: &HashMap<BigInt, Box<DataType>>|
                parse_hashmap! { color_hashmap |> Color {
                    a |> 0 => DataType::VInt => id,
                    r |> 0 => DataType::VInt => id,
                    g |> 0 => DataType::VInt => id,
                    b |> 0 => DataType::VInt => id,
                }},
                control |>  4 => DataType::VInt => id,
                team |>     5 => DataType::VInt => id,
                handicap |> 6 => DataType::VInt => id,
                observe |>  7 => DataType::VInt => id,
                result |>   8 => DataType::VInt => id,
            }
        }
    }
    players.iter().map(|p| typed_expect!(&**p, DataType::HashMap, parse_player)).collect()
}

pub fn parse_details(details: HashMap<BigInt, Box<DataType>>) -> Details {
    parse_hashmap! {
        details |> Details {
            players            |> 0  => DataType::Array   => parse_players,
            map_name           |> 1  => DataType::Blob    => decode_blob,
            difficulty         |> 2  => DataType::Blob    => decode_blob,
            thumbnail          |> 3  => DataType::HashMap => parse_thumbnail,
            blizz_map          |> 4  => DataType::UInt8   => id,
            file_time          |> 5  => DataType::VInt    => id,
            utc_adjustment     |> 6  => DataType::VInt    => id,
            description        |> 7  => DataType::Blob    => decode_blob,
            image_file_path    |> 8  => DataType::Blob    => decode_blob,
            map_file_name      |> 9  => DataType::Blob    => decode_blob,
            cache_handle       |> 10 => DataType::Array   => parse_cache_handle,
            mini_save          |> 11 => DataType::UInt8   => id,
            game_speed         |> 12 => DataType::VInt    => id,
            default_difficulty |> 13 => DataType::VInt    => id,
        }
    }
}
