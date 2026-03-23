
use super::structs::*;
use super::utils::*;

use crate::sc2mpq::parser::*;

use nom::{
    bytes::complete::{ take, tag },
    combinator::map,
    number::complete::*,
    multi::count,
    IResult,
};

use flate2::read::ZlibDecoder;
use bzip2::read::BzDecoder;
use std::io::Read;

fn to_u64_le(a: &[u8]) -> u64 {
    let fill = 8 - a.len();
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
        file_size,
        flags
    }))
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


fn decompress(block: &[u8]) -> Result<Vec<u8>, FileReadError> {
    let (mut block, compression_header) = le_u8_as_u64(block).unwrap();
    match compression_header {
        0 => Ok(Vec::from(block)), // No compression
        2 => { // zlib
            let mut z = ZlibDecoder::new(&mut block);
            let mut res = Vec::new();
            let _ = z.read_to_end(&mut res).unwrap();
            Ok(res)
        },
        16 => { // bz2
            let mut decompressor = BzDecoder::new(block);
            let mut content: Vec<u8> = Vec::new();
            match decompressor.read_to_end(&mut content) {
                Ok(_status) => {
                    Ok(content)
                },
                Err(e) => {
                    println!("error : {:?}", e);
                    Err(FileReadError::FailedDecompression)
                }
            }
        }
        i => Err(FileReadError::UnknownCompression(i))
    }
}


pub fn read_file(data: &[u8],
                 header: &MPQHeader,
                 bt_entry: &BlockTableEntry, _ht_entry: &HashTableEntry,
                 force_decompress: bool) -> Result<Vec<u8>, FileReadError> {
    if bt_entry.flags & BlockFlag::IsFile      == 0 { return Err(FileReadError::NotAFile); }
    if bt_entry.flags & BlockFlag::IsEncrypted != 0 { return Err(FileReadError::EncryptionNotImplemented) }
    if bt_entry.block_size == 0                     { return Err(FileReadError::ZeroSizedFile); }

    let offset = bt_entry.block_offset as usize;
    println!("Offset: {}, block size: {}, data size: {}", offset, bt_entry.block_size, data.len());
    let block_end = offset + (bt_entry.block_size as usize);
    let block = &data[offset..block_end];

    if bt_entry.flags & (BlockFlag::IsUnit as u32) != 0 {
        let is_compressed = bt_entry.flags & (BlockFlag::IsCompressed as u32) != 0
                            && (force_decompress || bt_entry.file_size > bt_entry.block_size);
        println!("Unit file, is compressed : {}", is_compressed);
        if is_compressed {
            decompress(block)
        } else {
            Ok(Vec::from(block))
        }
    } else {
        let sector_size = 512 << header.sector_size_shift;
        println!("sector size: 512 << sector_size_shift={} == {}", header.sector_size_shift, sector_size);
        let sector = ( bt_entry.block_size / sector_size ) + 1;
        println!("sector: block_size={} / sector_size={} + 1 == {}", bt_entry.block_size, sector_size, sector);
        let mut sector: usize = sector as usize;
        let crc = if bt_entry.flags & BlockFlag::FileSectorCRC == 1{
            sector += 1;
            true
        } else {
            false
        };
        fn test_le_u32(data: &[u8]) -> IResult<&[u8], u32> {
            le_u32(data)
        }
        let rev_block: Vec<u8> = block[0..((sector+1)*4)].iter().rev().copied().collect::<Vec<u8>>();
        let (_, positions) = map(count(test_le_u32, sector + 1), Vec::from)(&rev_block).unwrap();
        println!("Positions: {:?} <-> {:?}", positions, rev_block);
        let mut sector_bytes_left = bt_entry.block_size;
        let range = positions.len() - (if crc { 2 } else { 1 });
        let mut result = Vec::new();
        for i in 0..range {
            println!("Sector index: {}, sector position: {}..{},",
                     i, positions[i], positions[i+1]);
            let mut sector = Vec::from(&block[(positions[i] as usize)..(positions[i+1] as usize)]);
            let is_compressed = bt_entry.flags & (BlockFlag::IsCompressed as u32) != 0
                && (force_decompress || sector_bytes_left > sector.len().try_into().unwrap());
            if is_compressed {
                sector = decompress(&sector)?;
            }
            sector_bytes_left -= sector.len() as u32;
            result.append(&mut sector);
        }
        Ok(result)
    }
}

pub fn find_hash_entry_by_name<'a>(name: String,
                           entries: &'a Vec<HashTableEntry>,
                           crypttable: &CryptTable) -> Option<&'a HashTableEntry> {
    let hash_a = crypttable.hash(name.clone(), HashType::MPQHashNameA);
    let hash_b = crypttable.hash(name.clone(), HashType::MPQHashNameB);
    println!("Hash A : {} -> {:#x} ({})", name, hash_a, hash_a);
    println!("Hash B : {} -> {:#x} ({})", name, hash_b, hash_b);
    entries.iter().find(|e| u64::from(e.filepath_hash_a) == hash_a && u64::from(e.filepath_hash_b) == hash_b)
}
