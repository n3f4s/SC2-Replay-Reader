use num_bigint::{ BigInt, BigUint };
use std::collections::HashMap;

#[derive(Clone)]
pub enum DataType {
    Array(Vec<Box<DataType>>),
    VInt(BigInt),
    Blob(Vec<u8>),
    Optional(Option<Box<DataType>>),
    HashMap(HashMap<BigInt, Box<DataType>>),
    UInt8(u8),
    UInt32(u32),
    UInt64(u64),
}

impl DataType {
    fn pretty_print(&self, level: u64) -> String {
        use DataType::*;
        match self { // FIXME handle level
            UInt8(i)  => format!("{}", i),
            UInt32(i) => format!("{}", i),
            UInt64(i) => format!("{}", i),
            Blob(v) => {
                let b = BigUint::from_bytes_le(v);
                format!("{}", b)
            }
            VInt(v) => format!("{}", v),
            Optional(Some(b)) => format!("Some({})", b.pretty_print(level)),
            Optional(None) => String::from("None"),
            Array(vec) => {
                let str_ = vec.iter().map(|d| d.pretty_print(level+1) + ",\n").collect::<Vec<String>>().join("");
                format!("[\n{}]", str_)
            },
            HashMap(hash) => {
                let str_ = hash
                    .iter()
                    .map(|( k, v )| format!("{}: {},\n",
                                            k,
                                            v.pretty_print(level+2))).collect::<Vec<String>>().join("");
                format!("{{\n{}}}", str_)
            }
        }
    }
}
