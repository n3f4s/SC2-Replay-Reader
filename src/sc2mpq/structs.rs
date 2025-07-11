
use std::fmt;
use std::collections::HashMap;
use std::iter::{Map, zip};

use encoding::{ all::ISO_8859_1, all::WINDOWS_1252, DecoderTrap, Encoding };
use num_bigint::{ BigUint, ToBigUint };

#[derive(Debug)]
pub enum DataType {
    ByteString(String),
    ArrayObject(Vec<Box<DataType>>),
    KeyValueObject(HashMap<u8, Box<DataType>>),
    SingleByteInteger(i8),
    FourBytesInteger(i64),
    VLFInteger(BigUint),
}

impl DataType {
    pub fn bytestring(s: String) -> DataType { DataType::ByteString(s) }
    pub fn array_object(v: Vec<Box<DataType>>) -> DataType { DataType::ArrayObject(v) }
    pub fn singlebyte_int(i: u8) -> DataType {
        let sign = i & 0x1; // The sign bit is the first bit
        let v: i8 = (i >> 1).try_into().unwrap();
        let v = v * (if sign != 0 { -1 } else { 1 });
        DataType::SingleByteInteger(v)
    }
    pub fn fourbytes_int(i: &[u8]) -> DataType {
        let sign = i[3] & 0x1;
        let v = (i[3] as u64)
              + ((i[2] as u64) << 8)
              + ((i[1] as u64) << 16)
              + ((i[0] as u64) << 24);
        let v: i64 = (v >> 1).try_into().unwrap();
        let v = v * (if sign != 0 { -1 } else { 1 });
        DataType::FourBytesInteger(v)
    }
    pub fn vlf_int(v: Vec<u8>) -> DataType {
        /**
        Repack the bytes in order to have 8 bits bytes instead
        of 7 bits + continuation bit of the serialized data
        */
        let mut a = v[0].to_biguint().unwrap();
        for i in 1..v.len() {
            a <<= 7;
            a += v[i];
        }
        DataType::VLFInteger(a)
    }

    pub fn pretty_print(&self, indent: usize) -> String {
        match self {
            DataType::ByteString(i)        => format!("\"{}\"", i),
            DataType::SingleByteInteger(i) => format!("{}", i),
            DataType::FourBytesInteger(i)  => format!("{}", i),
            DataType::VLFInteger(i)        => format!("{}", i),
            DataType::ArrayObject(a) => {
                let mut s = "[ ".to_string();
                for i in 0..a.len()-1 {
                    s += &format!("{}, ", a[i].pretty_print(indent+1));
                }
                s += &format!("{} ]", a[a.len()-1]);
                s
            }
            DataType::KeyValueObject(a) => {
                let mut s = "{".to_string();
                let mut keys: Vec<&u8> = a.keys().collect();
                keys.sort();
                for i in 0..keys.len()-1 {
                    s += &format!("\n{}\"{}\": {},",
                                  "\t".repeat(indent+1),
                                  keys[i],
                                  a[&keys[i]].pretty_print(indent+1));
                }
                s += &format!("\n{}\"{}\": {}",
                              "\t".repeat(indent+1),
                              keys[keys.len()-1],
                              a[&keys[keys.len()-1]].pretty_print(indent+1));
                s += &("\n".to_owned() + &"\t".repeat(indent) + "}");
                s
            }
        }
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.pretty_print(0))
    }
}

impl PartialEq for DataType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (DataType::ByteString(a), DataType::ByteString(b)) => a == b,
            (DataType::ArrayObject(a), DataType::ArrayObject(b)) => zip(a, b).all(|(a, b)| a.eq(&*b)),
            (DataType::SingleByteInteger(a), DataType::SingleByteInteger(b)) => a == b,
            (DataType::FourBytesInteger(a), DataType::FourBytesInteger(b)) => a == b,
            _ => false // FIXME do VLFI and KV
        }
    }
}
