
use super::structs::*;

use crate::sc2mpq::parser::*;

use nom::{
    bytes::complete::{ take, tag },
    combinator::map,
    number::complete::*,
    multi::count,
    IResult,
};

fn to_u64_le(a: &[u8]) -> u64 {
    let mut fill = 8 - a.len();
    let a = [a, &vec![0; fill]].concat();
    u64::from_le_bytes(a.try_into().expect("Wrong size"))
}


pub fn le_u32_as_u64(data: &[u8]) -> IResult<&[u8], u64> {
    map(le_u32, |i| i as u64)(data)
}
pub fn le_u16_as_u64(data: &[u8]) -> IResult<&[u8], u64> {
    map(le_u16, |i| i as u64)(data)
}
pub fn le_u8_as_u64(data: &[u8]) -> IResult<&[u8], u64> {
    map(le_u8, |i| i as u64)(data)
}

pub fn parse_sc2mpq_header(data: &[u8]) -> IResult<&[u8], SC2MPQHeader> {
    let ( data, _        ) = tag([0x4d, 0x50, 0x51, 0x1b])(data)?;
    // FIXME use to_*_as_64
    let ( data, max_size ) = map(take(4_u8), to_u64_le)(data)?;
    let ( data, offset   ) = map(take(4_u8), to_u64_le)(data)?;
    let ( data, size     ) = map(take(4_u8), to_u64_le)(data)?;
    let ( data, kv       ) = parse_serialized_data(data)?;
    Ok((data, SC2MPQHeader::new(max_size, offset, size, kv).unwrap()))
}

pub fn parse_mpq_header(data: &[u8]) -> IResult<&[u8], MPQHeader> {
    // In comment: the address of the value (from the beginning of the header)
    let (data, _) = tag([0x4d, 0x50, 0x51, 0x1a])(data)?;  // 0x00
    let (data, header_size)        = le_u32_as_u64(data)?; // 0x04
    let (data, archive_size)       = le_u32_as_u64(data)?; // 0x08
    let (data, format_version)     = le_u16_as_u64(data)?; // 0x0C
    let (data, sector_size_shift)  = le_u8_as_u64(data)?;  // 0x0E
    let (data, _)                  = le_u8_as_u64(data)?;  // Padding byte
    let (data, hashtable_offset)   = le_u32_as_u64(data)?; // 0x10
    let (data, blocktable_offset)  = le_u32_as_u64(data)?; // 0x14
    let (data, hashtable_entries)  = le_u32_as_u64(data)?; // 0x18
    let (data, blocktable_entries) = le_u32_as_u64(data)?; // 0x1C
    let (data,
         extended_blocktable_offset,
         hashtable_offset_high,
         blocktable_offset_high ) = if format_version == 1 {
        // FIXME use the other versions like above
        let (data, extended_blocktable_offset) = map(take(8u8), to_u64_le)(data)?;
        let (data, hashtable_offset_high)      = map(take(2u8), to_u64_le)(data)?;
        let (data, blocktable_offset_high)     = map(take(2u8), to_u64_le)(data)?;
        (data, extended_blocktable_offset, hashtable_offset_high, blocktable_offset_high)
    } else { (data, 0, 0, 0) };
    Ok((data, MPQHeader {
        header_size: header_size,
        archive_size: archive_size,
        // FIXME there's more version available
        format_version: format_version, // if format_version == 1 { MPQFormatVersion::BurningCrusade }
        // else { MPQFormatVersion::Original },
        sector_size_shift: sector_size_shift,
        hashtable_offset: ( hashtable_offset_high << 32 ) + hashtable_offset,
        blocktable_offset: ( blocktable_offset_high << 32 ) + blocktable_offset,
        hashtable_entries: hashtable_entries,
        blocktable_entries: blocktable_entries,
        extended_blocktable_offset: extended_blocktable_offset,
    }))
}

fn parse_blocktable_entry(data: &[u8]) -> IResult<&[u8], BlockTableEntry> {
    let (data, offset) = le_u32(data)?;
    let (data, size) = le_u32(data)?;
    let (data, file_size) = le_u32(data)?;
    let (data, flags) = le_u32(data)?;
    Ok((data, BlockTableEntry {
        block_offset: offset,
        block_size: size,
        file_size: file_size,
        flags: flags
    }))
}

pub fn parse_packed_blocktable(data: &[u64]) -> Vec<BlockTableEntry> {
    let mut res = Vec::new();
    let mut i = 0;
    while i < data.len() {
        res.push(BlockTableEntry {
            block_offset: (data[i] >> 32) as u32,
            block_size: (data[i] & ((1 << 32)-1)) as u32,
            file_size: (data[i+1] >> 32) as u32,
            flags: (data[i+1] & ((1 << 32)-1)) as u32,
        });
        i += 2;
    }
    res
}

pub fn parse_blocktable(data: &[u8], size: usize) -> IResult<&[u8], Vec<BlockTableEntry>> {
    count(parse_blocktable_entry, size)(data)
}

pub fn parse_hashtable_entry(data: &[u8]) -> IResult<&[u8], HashTableEntry> {
    const EMPTY_FILE: u32 = FileMissingFlag::Empty as u32;
    const DELETED_FILE: u32 = FileMissingFlag::Deleted as u32;

    let (data, fp_hash_a) = le_u32(data)?;
    let (data, fp_hash_b) = le_u32(data)?;
    let (data, langid)    = le_u16(data)?;
    let (data, platform)  = le_u8(data)?;
    let (data, _)         = le_u8(data)?; // padding byte
    let (data, fb_idx)    = le_u32(data)?;
    Ok((data, HashTableEntry {
        filepath_hash_a: fp_hash_a,
        filepath_hash_b: fp_hash_b,
        language: langid,
        platform_id: platform,
        file_block_index: match fb_idx {
            EMPTY_FILE   => FileBlockIndex::FileMissing(FileMissingFlag::Empty),
            DELETED_FILE => FileBlockIndex::FileMissing(FileMissingFlag::Deleted),
            _            => FileBlockIndex::FilePresent(fb_idx),
        }
    }))
}

pub fn parse_hashtable(data: &[u8], size: usize) -> IResult<&[u8], Vec<HashTableEntry>> {
    count(parse_hashtable_entry, size)(data)
}

pub fn read_file<'a>(data: &'a [u8], index_compensator: fn(usize) -> usize,
                     bt_entry: &BlockTableEntry, _ht_entry: &HashTableEntry,
                     force_decompress: bool) -> Result<Vec<u8>, FileReadError> {
    if bt_entry.flags & (BlockFlag::IsFile as u32) == 0      { return Err(FileReadError::NotAFile); }
    if bt_entry.flags & (BlockFlag::IsEncrypted as u32) != 0 { return Err(FileReadError::EncryptionNotImplemented) }
    if bt_entry.block_size == 0                              { return Err(FileReadError::ZeroSizedFile); }
    let offset = index_compensator(bt_entry.block_offset as usize);
    let block = &data[offset..bt_entry.block_size as usize];
    if bt_entry.flags & (BlockFlag::IsUnit as u32) == 1 {
        let is_compressed = bt_entry.flags & (BlockFlag::IsCompressed as u32) != 0
                            && (force_decompress || bt_entry.file_size > bt_entry.block_size);
        if is_compressed {
            // FIXME implement decryption
            Ok(Vec::from(block))
        } else {
            Ok(Vec::from(block))
        }
    } else {
        panic!("Not implemented");
    }
}
