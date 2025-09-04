extern crate encoding;
extern crate num_bigint;

mod mpq;
mod sc2mpq;
mod utils;

use std::{ env, fs };

use mpq::{ parser::*, structs::*, utils::* };
use utils::*;

fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let args: Vec<String> = env::args().collect();
    let data: Vec<u8> = fs::read(args[1].clone())?;
    let file_size = data.len();
    let (data, sc2header) = parse_sc2mpq_header(&data).unwrap();
    println!("{}", sc2header);
    let data = jump_to_offset(sc2header.header_offset as usize, file_size, data.len(), data);
    // let mpq_archive_size = data.len();
    let (data, mpqheader) = parse_mpq_header(data).unwrap();
    println!("{}", mpqheader);

    // FIXME: it's a test for blocktable reading, read stuff before the blocktable before once test are done
    // let data = jump_to_offset(mpqheader.blocktable_offset as usize, mpq_archive_size, data.len(), &data);
    let compensated_blocktable_offset = ((mpqheader.blocktable_offset + sc2header.header_offset) as usize - (file_size - data.len()));
    let blocktable_end = compensated_blocktable_offset + mpqheader.blocktable_byte_size() as usize;

    let compensated_hashtable_offset = ((mpqheader.hashtable_offset as usize + sc2header.header_offset as usize) - (file_size - data.len()));
    let hashtable_end = compensated_hashtable_offset + mpqheader.hashtable_byte_size() as usize;

    println!("current file size: {}", data.len());
    let encrypted_blocktable = &data[compensated_blocktable_offset..blocktable_end];
    let crypttable = CryptTable::new();
    let blocktable_key = crypttable.hash("(block table)".to_string(), HashType::MPQHashFileKey);
    let blocktable = crypttable.decrypt(&pack_u8_vec(encrypted_blocktable), blocktable_key);
    // let parsed_blocktable = parse_packed_blocktable(&blocktable);
    let unpacked_blocktable = unpack_u64_vec(&blocktable);
    let ( _, parsed_blocktable ) = parse_blocktable(&unpacked_blocktable, mpqheader.blocktable_entries as usize).unwrap();
    println!("offset\tblock size\tfile size\tflags");
    for block in parsed_blocktable {
        println!("{:#x}\t{}\t{}\t{:x}", block.block_offset, block.block_size, block.file_size, block.flags);
    }

    println!("current file size: {}, hashtable start: {}, hashtable end: {}", data.len(), compensated_hashtable_offset, hashtable_end);
    let encrypted_hashtable = &data[compensated_hashtable_offset..hashtable_end];
    let hashtable_key = crypttable.hash("(hash table)".to_string(), HashType::MPQHashFileKey);
    let hashtable = crypttable.decrypt(&pack_u8_vec(encrypted_hashtable), hashtable_key);
    let unpacked_hashtable = unpack_u64_vec(&hashtable);
    let ( _, parsed_hashtable ) = parse_hashtable(&unpacked_hashtable, mpqheader.hashtable_entries as usize).unwrap();
    println!("Hash A\tHash B\tLocl\tPlat\tBlockIdx");
    for entry in parsed_hashtable {
        println!("{:#x}\t{:#x}\t{:#x}\t{:#x}\t{:#x}\t",
                 entry.filepath_hash_a,
                 entry.filepath_hash_b,
                 entry.language,
                 entry.platform_id,
                 match entry.file_block_index {
                     FileBlockIndex::FilePresent(i) => i,
                     FileBlockIndex::FileMissing(i) => i as u32,
                 }
        );
    }
    Ok(())
}
