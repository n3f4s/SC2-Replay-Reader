use num_bigint::{ BigInt, BigUint };
use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Debug)]
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
    fn pretty_print(&self, level: usize) -> String {
        use DataType::*;
        match self { // FIXME handle level
            UInt8(i)  => format!("{}_u8", i),
            UInt32(i) => format!("{}_u32", i),
            UInt64(i) => format!("{}_u64", i),
            Blob(v) => {
                let b = BigUint::from_bytes_le(v);
                format!("{}_bytes", b)
            }
            VInt(v) => format!("{}_vint", v),
            Optional(Some(b)) => format!("Some({})", b.pretty_print(level)),
            Optional(None) => String::from("None"),
            Array(vec) => {
                let str_ = vec.iter().map(|d| "  ".repeat(level) + &d.pretty_print(level+1) + ",\n").collect::<Vec<String>>().join("");
                format!("[\n{}{}]", str_, "  ".repeat(level))
            },
            HashMap(hash) => {
                let str_ = hash
                    .iter()
                    .map(|( k, v )| format!("{}{}: {},\n",
                                            "  ".repeat(level+1),
                                            k,
                                            v.pretty_print(level+2))).collect::<Vec<String>>().join("");
                format!("{{\n{}{}}}", str_, "  ".repeat(if level > 0 { level-1 } else { level }))
            }
        }
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.pretty_print(0))
    }
}
