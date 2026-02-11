
use std::{ fmt };

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

    #[allow(dead_code)]
    pub raw_kv_header: DataType,
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
    /// Block is a file, and follows the file data format; otherwise, block is free space or unused. If the block is not a file, all other flags should be cleared, and FileSize should be 0.
    IsFile             = 0x80000000,
    /// File is stored as a single unit, rather than split into sectors.
    IsUnit             = 0x01000000,
    /// The file's encryption key is adjusted by the block offset and file size (explained in detail in the File Data section). File must be encrypted.
    AdjustedEncryptKey = 0x00020000,
    /// File is encrypted.
    IsEncrypted        = 0x00010000,
    /// File is compressed. File cannot be imploded.
    IsCompressed       = 0x00000200,
    /// File is imploded. File cannot be compressed.
    IsImploded         = 0x00000100,
    /// ???
    FileSectorCRC      = 0x04000000,
}

#[allow(dead_code)]
pub enum HashType {
    MPQHashtableOffset = 0,
    MPQHashNameA       = 1,
    MPQHashNameB       = 2,
    MPQHashFileKey     = 3,
}

pub struct BlockTableEntry {
    pub block_offset: u32,
    pub block_size: u32,
    pub file_size: u32, // If file is compressed then file_size >= block_size
    pub flags: u32,
}

#[derive(Clone, Copy)]
pub enum FileMissingFlag {
    Empty   = 0xFFFFFFFF,
    Deleted = 0xFFFFFFFE,
}

#[derive(Clone, Copy)]
pub enum FileBlockIndex {
    FilePresent(u32),
    FileMissing(FileMissingFlag),
}

pub struct HashTableEntry {
    pub filepath_hash_a: u32,
    pub filepath_hash_b: u32,
    pub language: u16, // Uses MS LANGID format,
    pub platform_id: u8,
    pub file_block_index: FileBlockIndex,
}

pub enum FileReadError {
    EncryptionNotImplemented,
    NotAFile,
    ZeroSizedFile,
    UnknownCompression(u64),
    FailedDecompression,
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
    pub fn hashtable_byte_size(&self) -> u64 {
        // An entry is 16 bytes
        self.hashtable_entries * 16
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

impl std::ops::BitAnd<BlockFlag> for u32 {
    type Output = Self;

    fn bitand(self, rhs: BlockFlag) -> Self::Output {
        self & (rhs as u32)
    }
}
impl std::ops::BitOr<BlockFlag> for u32 {
    type Output = Self;

    fn bitor(self, rhs: BlockFlag) -> Self::Output {
        self | (rhs as u32)
    }
}

impl From<FileMissingFlag> for u32 {
    fn from(value: FileMissingFlag) -> u32 {
        value as u32
    }
}
