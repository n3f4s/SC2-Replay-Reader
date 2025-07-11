
use std::{ fmt, ops };

use crate::sc2mpq::structs::{*};

pub struct GameVersion {
    pub major: i64,
    pub minor: i64,
    pub patch: i64,
    pub build: i64,
}

pub struct SC2MPQHeader {
    pub max_data_size: u64,
    pub header_offset: u64,
    pub data_size: u64,

    pub game_version: GameVersion,
    pub game_length: u64,

    pub raw_kv_header: DataType,
}

#[derive(Debug)]
pub enum MPQFormatVersion {
    Original, BurningCrusade,
}

#[derive(Debug)]
pub struct MPQHeader {
    // Offset are relative to the beginning of the archive (aka, beginning of the normal MPQ header)
    pub header_size: u64,
    pub archive_size: u64,
    pub format_version: u64,
    pub sector_size_shift: u64,
    pub hashtable_offset: u64,
    pub hashtable_entries: u64,
    pub blocktable_offset: u64,
    pub blocktable_entries: u64,
    pub extended_blocktable_offset: u64
}

pub enum BlockFlag {
    IsFile             = 0x80000000,
    IsUnit             = 0x01000000,
    AdjustedEncryptKey = 0x00020000,
    IsEncrypted        = 0x00010000,
    IsCompressed       = 0x00000200,
    IsImploded         = 00000100,
}

pub enum HashType {
    MPQ_hashtable_offset = 0,
    MPQ_hash_name_A      = 1,
    MPQ_hash_name_B      = 2,
    MPQ_hash_file_key    = 3,
}

pub struct BlockTableEntry {
    pub block_offset: u32,
    pub block_size: u32,
    pub file_size: u32, // If file is compressed then file_size =/= block_size
    pub flags: u32,
}

fn extract_number(data: &DataType) -> Option<i64> {
    match data {
        DataType::FourBytesInteger(i) => Some(*i),
        DataType::SingleByteInteger(i) => Some(*i as i64),
        DataType::VLFInteger(i) => {
            let digits = i.to_u64_digits();
            Some( if digits.len() > 0 { digits[0].try_into().unwrap() } else { 0 } )
        },
        _ => None,
    }
}

impl GameVersion {
    fn from_datatype(data: &DataType) -> Option<GameVersion> {
        match data {
            DataType::KeyValueObject(hashmap) => Some(GameVersion {
                major: extract_number(&hashmap[&1])?,
                minor: extract_number(&hashmap[&2])?,
                patch: extract_number(&hashmap[&3])?,
                build: extract_number(&hashmap[&4])?,
            }),
            _ => None
        }
    }
}

impl MPQHeader {
    pub fn blocktable_byte_size(&self) -> u64 {
        // An entry is 4 u32
        self.blocktable_entries * 4 * 4
    }
}

impl SC2MPQHeader {
    pub fn new(max_size: u64, offset: u64, size: u64, kv: DataType) -> Option<SC2MPQHeader> {
        match kv {
            DataType::KeyValueObject(ref hashmap) => {
                let version = GameVersion::from_datatype(&hashmap[&1])?;
                Some(SC2MPQHeader {
                    max_data_size: max_size,
                    header_offset: offset,
                    data_size: size,

                    game_version: version,
                    game_length: extract_number(&hashmap[&3])?.abs() as u64,

                    raw_kv_header: kv
                }
            ) },
            _ => None
        }
    }
}

impl fmt::Display for GameVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}.{}", self.major, self.minor, self.patch, self.build)
    }
}

impl fmt::Display for MPQFormatVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MPQFormatVersion::Original => write!(f, "Original"),
            MPQFormatVersion::BurningCrusade => write!(f, "Burning Crusade"),
        }
    }
}

impl fmt::Display for SC2MPQHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"SC2 MPQ Header\": {{")?;
        write!(f, "\n\t\"game version\":                 {},", self.game_version)?;
        write!(f, "\n\t\"game length (in frames)\":      {},", self.game_length)?;
        write!(f, "\n\t\"header offset\":                {},", self.header_offset)?;
        write!(f, "\n\t\"data size\":                    {},", self.data_size)?;
        write!(f, "\n\t\"max data size\":                {}", self.max_data_size)?;
        write!(f, "\n}}")
    }
}

impl fmt::Display for MPQHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"MPQ Header\": {{")?;
        write!(f, "\n\t\"header size\":                  {},", self.header_size)?;
        write!(f, "\n\t\"archive size\":                 {},", self.archive_size)?;
        write!(f, "\n\t\"format version\":               {},", self.format_version)?;
        write!(f, "\n\t\"sector size shift\":            {},", self.sector_size_shift)?;
        write!(f, "\n\t\"hashtable size (in entries)\":  {},", self.hashtable_entries)?;
        write!(f, "\n\t\"hashtable offset\":             {},", self.hashtable_offset)?;
        write!(f, "\n\t\"blocktable size (in entries)\": {},", self.blocktable_entries)?;
        write!(f, "\n\t\"blocktable offset\":            {},", self.blocktable_offset)?;
        write!(f, "\n\t\"extended blocktable offset\":   {}", self.extended_blocktable_offset)?;
        write!(f, "\n}}")
    }
}
